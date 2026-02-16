use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use crossbeam::channel::{Receiver, Sender, unbounded};
use globset::GlobSet;
use walkdir::{DirEntry, WalkDir};

type ProcFilesFunction<Config> = dyn Fn(PathBuf, &Config) -> std::io::Result<()> + Send + Sync;

type ProcDirPathsFunction<Config> =
    dyn Fn(&mut HashMap<String, Vec<PathBuf>>, &Path, &Config) + Send + Sync;

type ProcPathFunction<Config> = dyn Fn(&Path, &Config) + Send + Sync;

// Null functions removed at compile time
fn null_proc_dir_paths<Config>(_: &mut HashMap<String, Vec<PathBuf>>, _: &Path, _: &Config) {}
fn null_proc_path<Config>(_: &Path, _: &Config) {}

#[derive(Debug)]
struct JobItem<Config> {
    path: PathBuf,
    cfg: Arc<Config>,
}

type JobReceiver<Config> = Receiver<Option<JobItem<Config>>>;
type JobSender<Config> = Sender<Option<JobItem<Config>>>;

// Both args are moved into this thread entry point from a `move ||` closure;
// pass-by-value is required because `Receiver` is consumed and `Arc` is moved.
#[allow(clippy::needless_pass_by_value)]
fn consumer<Config, ProcFiles>(receiver: JobReceiver<Config>, func: Arc<ProcFiles>)
where
    ProcFiles: Fn(PathBuf, &Config) -> std::io::Result<()> + Send + Sync,
{
    while let Ok(job) = receiver.recv() {
        if job.is_none() {
            break;
        }
        // Cannot panic because of the check immediately above.
        let job = job.unwrap();
        let path = job.path.clone();

        if let Err(err) = func(job.path, &job.cfg) {
            log::error!("{err:?} for file {path:?}");
        }
    }
}

fn send_file<T>(
    path: PathBuf,
    cfg: &Arc<T>,
    sender: &JobSender<T>,
) -> Result<(), ConcurrentErrors> {
    sender
        .send(Some(JobItem {
            path,
            cfg: Arc::clone(cfg),
        }))
        .map_err(|e| ConcurrentErrors::Sender(e.to_string()))
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'))
}

fn explore<Config, ProcDirPaths, ProcPath>(
    files_data: FilesData,
    cfg: &Arc<Config>,
    proc_dir_paths: ProcDirPaths,
    proc_path: ProcPath,
    sender: &JobSender<Config>,
) -> Result<HashMap<String, Vec<PathBuf>>, ConcurrentErrors>
where
    ProcDirPaths: Fn(&mut HashMap<String, Vec<PathBuf>>, &Path, &Config) + Send + Sync,
    ProcPath: Fn(&Path, &Config) + Send + Sync,
{
    let FilesData {
        paths,
        ref include,
        ref exclude,
    } = files_data;

    let mut all_files: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for path in paths {
        if !path.exists() {
            log::warn!("File doesn't exist: {path:?}");
            continue;
        }
        if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| !is_hidden(e))
            {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => return Err(ConcurrentErrors::Sender(e.to_string())),
                };
                let path = entry.path().to_path_buf();
                if (include.is_empty() || include.is_match(&path))
                    && (exclude.is_empty() || !exclude.is_match(&path))
                    && path.is_file()
                {
                    proc_dir_paths(&mut all_files, &path, cfg);
                    send_file(path, cfg, sender)?;
                }
            }
        } else if (include.is_empty() || include.is_match(&path))
            && (exclude.is_empty() || !exclude.is_match(&path))
            && path.is_file()
        {
            proc_path(&path, cfg);
            send_file(path, cfg, sender)?;
        }
    }

    Ok(all_files)
}

/// Series of errors that might happen when processing files concurrently.
#[derive(Debug)]
pub(crate) enum ConcurrentErrors {
    /// Producer side error.
    ///
    /// An error occurred inside the producer thread.
    Producer(String),
    /// Sender side error.
    ///
    /// An error occurred when sending an item.
    Sender(String),
    /// Receiver side error.
    ///
    /// An error occurred inside one of the receiver threads.
    Receiver(String),
    /// Thread side error.
    ///
    /// A general error occurred when a thread is being spawned or run.
    Thread(String),
}

impl std::fmt::Display for ConcurrentErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Producer(msg) => write!(f, "producer error: {msg}"),
            Self::Sender(msg) => write!(f, "sender error: {msg}"),
            Self::Receiver(msg) => write!(f, "receiver error: {msg}"),
            Self::Thread(msg) => write!(f, "thread error: {msg}"),
        }
    }
}

impl std::error::Error for ConcurrentErrors {}

/// Data related to files.
#[derive(Debug)]
pub(crate) struct FilesData {
    /// Kind of files included in a search.
    pub(crate) include: GlobSet,
    /// Kind of files excluded from a search.
    pub(crate) exclude: GlobSet,
    /// List of file paths.
    pub(crate) paths: Vec<PathBuf>,
}

/// A runner to process files concurrently.
pub(crate) struct ConcurrentRunner<Config> {
    proc_files: Box<ProcFilesFunction<Config>>,
    proc_dir_paths: Box<ProcDirPathsFunction<Config>>,
    proc_path: Box<ProcPathFunction<Config>>,
    num_jobs: usize,
}

impl<Config> std::fmt::Debug for ConcurrentRunner<Config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConcurrentRunner")
            .field("num_jobs", &self.num_jobs)
            .finish_non_exhaustive()
    }
}

impl<Config: 'static + Send + Sync> ConcurrentRunner<Config> {
    /// Creates a new `ConcurrentRunner`.
    ///
    /// * `num_jobs` - Number of jobs utilized to process files concurrently.
    /// * `proc_files` - Function that processes each file found during
    ///   the search.
    pub(crate) fn new<ProcFiles>(num_jobs: usize, proc_files: ProcFiles) -> Self
    where
        ProcFiles: 'static + Fn(PathBuf, &Config) -> std::io::Result<()> + Send + Sync,
    {
        let num_jobs = std::cmp::max(2, num_jobs) - 1;
        Self {
            proc_files: Box::new(proc_files),
            proc_dir_paths: Box::new(null_proc_dir_paths),
            proc_path: Box::new(null_proc_path),
            num_jobs,
        }
    }

    /// Runs the producer-consumer approach to process the files
    /// contained in a directory and in its own subdirectories.
    ///
    /// * `config` - Information used to process a file.
    /// * `files_data` - Information about the files to be included or excluded
    ///   from a search more the number of paths considered in the search.
    pub(crate) fn run(
        self,
        config: Config,
        files_data: FilesData,
    ) -> Result<HashMap<String, Vec<PathBuf>>, ConcurrentErrors> {
        let cfg = Arc::new(config);

        let (sender, receiver) = unbounded();

        let producer = {
            let sender = sender.clone();

            match thread::Builder::new()
                .name(String::from("Producer"))
                .spawn(move || {
                    explore(
                        files_data,
                        &cfg,
                        self.proc_dir_paths,
                        self.proc_path,
                        &sender,
                    )
                }) {
                Ok(producer) => producer,
                Err(e) => return Err(ConcurrentErrors::Thread(e.to_string())),
            }
        };

        let mut receivers = Vec::with_capacity(self.num_jobs);
        let proc_files = Arc::new(self.proc_files);
        for i in 0..self.num_jobs {
            let receiver = receiver.clone();
            let proc_files = proc_files.clone();

            let t = match thread::Builder::new()
                .name(format!("Consumer {i}"))
                .spawn(move || {
                    consumer(receiver, proc_files);
                }) {
                Ok(receiver) => receiver,
                Err(e) => return Err(ConcurrentErrors::Thread(e.to_string())),
            };

            receivers.push(t);
        }

        let Ok(all_files) = producer.join() else {
            return Err(ConcurrentErrors::Producer(
                "Child thread panicked".to_owned(),
            ));
        };

        // Poison the receiver, now that the producer is finished.
        for _ in 0..self.num_jobs {
            if let Err(e) = sender.send(None) {
                return Err(ConcurrentErrors::Sender(e.to_string()));
            }
        }

        for receiver in receivers {
            if receiver.join().is_err() {
                return Err(ConcurrentErrors::Receiver(
                    "A thread used to process a file panicked".to_owned(),
                ));
            }
        }

        all_files
    }
}
