#[cfg(test)]
use crate::legacy::spaces::{CodeMetrics, FuncSpace, metrics};
#[cfg(test)]
use crate::legacy::traits::ParserTrait;

#[cfg(test)]
pub(crate) fn check_func_space<T: ParserTrait, F: Fn(FuncSpace)>(
    source: &str,
    filename: &str,
    check: F,
) {
    let path = std::path::PathBuf::from(filename);
    let mut trimmed_bytes = source.trim_end().trim_matches('\n').as_bytes().to_vec();
    trimmed_bytes.push(b'\n');
    let parser = T::new(trimmed_bytes, &path, None);
    let func_space = metrics(&parser, &path).unwrap();

    check(func_space)
}

#[cfg(test)]
pub(crate) fn check_metrics<T: ParserTrait>(
    source: &str,
    filename: &str,
    check: fn(CodeMetrics) -> (),
) {
    check_func_space::<T, _>(source, filename, |func_space| check(func_space.metrics))
}
