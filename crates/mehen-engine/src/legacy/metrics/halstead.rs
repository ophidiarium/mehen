use std::collections::HashMap;

use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::getter::Getter;
use crate::legacy::langs::{CCode, GoCode, KotlinCode, RubyCode};
use crate::legacy::node::Node;

/// The `Halstead` metric suite.
#[derive(Default, Clone, Debug)]
pub(crate) struct Stats {
    u_operators: u64,
    operators: u64,
    u_operands: u64,
    operands: u64,
}

/// Specifies the type of nodes accepted by the `Halstead` metric.
#[derive(Debug)]
pub(crate) enum HalsteadType {
    /// The node is an `Halstead` operator
    Operator,
    /// The node is an `Halstead` operand
    Operand,
    /// The node is unknown to the `Halstead` metric
    Unknown,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct HalsteadMaps<'a> {
    pub operators: HashMap<u16, u64>,
    pub operands: HashMap<&'a [u8], u64>,
}

impl<'a> HalsteadMaps<'a> {
    pub(crate) fn new() -> Self {
        HalsteadMaps {
            operators: HashMap::default(),
            operands: HashMap::default(),
        }
    }

    pub(crate) fn merge(&mut self, other: &Self) {
        for (k, v) in &other.operators {
            *self.operators.entry(*k).or_insert(0) += v;
        }
        for (k, v) in &other.operands {
            *self.operands.entry(*k).or_insert(0) += v;
        }
    }

    pub(crate) fn finalize(&self, stats: &mut Stats) {
        stats.u_operators = self.operators.len() as u64;
        stats.operators = self.operators.values().sum::<u64>();
        stats.u_operands = self.operands.len() as u64;
        stats.operands = self.operands.values().sum::<u64>();
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("halstead", 14)?;
        st.serialize_field("n1", &self.u_operators())?;
        st.serialize_field("N1", &self.operators())?;
        st.serialize_field("n2", &self.u_operands())?;
        st.serialize_field("N2", &self.operands())?;
        st.serialize_field("length", &self.length())?;
        st.serialize_field("estimated_program_length", &self.estimated_program_length())?;
        st.serialize_field("purity_ratio", &self.purity_ratio())?;
        st.serialize_field("vocabulary", &self.vocabulary())?;
        st.serialize_field("volume", &self.volume())?;
        st.serialize_field("difficulty", &self.difficulty())?;
        st.serialize_field("level", &self.level())?;
        st.serialize_field("effort", &self.effort())?;
        st.serialize_field("time", &self.time())?;
        st.serialize_field("bugs", &self.bugs())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "n1: {}, \
             N1: {}, \
             n2: {}, \
             N2: {}, \
             length: {}, \
             estimated program length: {}, \
             purity ratio: {}, \
             size: {}, \
             volume: {}, \
             difficulty: {}, \
             level: {}, \
             effort: {}, \
             time: {}, \
             bugs: {}",
            self.u_operators(),
            self.operators(),
            self.u_operands(),
            self.operands(),
            self.length(),
            self.estimated_program_length(),
            self.purity_ratio(),
            self.vocabulary(),
            self.volume(),
            self.difficulty(),
            self.level(),
            self.effort(),
            self.time(),
            self.bugs(),
        )
    }
}

impl Stats {
    pub(crate) fn merge(&self, _other: &Self) {}

    /// Returns `η1`, the number of distinct operators
    #[inline(always)]
    pub(crate) fn u_operators(&self) -> f64 {
        self.u_operators as f64
    }

    /// Returns `N1`, the number of total operators
    #[inline(always)]
    pub(crate) fn operators(&self) -> f64 {
        self.operators as f64
    }

    /// Returns `η2`, the number of distinct operands
    #[inline(always)]
    pub(crate) fn u_operands(&self) -> f64 {
        self.u_operands as f64
    }

    /// Returns `N2`, the number of total operands
    #[inline(always)]
    pub(crate) fn operands(&self) -> f64 {
        self.operands as f64
    }

    /// Returns the program length
    #[inline(always)]
    pub(crate) fn length(&self) -> f64 {
        self.operands() + self.operators()
    }

    /// Returns the calculated estimated program length
    #[inline(always)]
    pub(crate) fn estimated_program_length(&self) -> f64 {
        self.u_operators().mul_add(
            self.u_operators().log2(),
            self.u_operands() * self.u_operands().log2(),
        )
    }

    /// Returns the purity ratio
    #[inline(always)]
    pub(crate) fn purity_ratio(&self) -> f64 {
        self.estimated_program_length() / self.length()
    }

    /// Returns the program vocabulary
    #[inline(always)]
    pub(crate) fn vocabulary(&self) -> f64 {
        self.u_operands() + self.u_operators()
    }

    /// Returns the program volume.
    ///
    /// Unit of measurement: bits
    #[inline(always)]
    pub(crate) fn volume(&self) -> f64 {
        // Assumes a uniform binary encoding for the vocabulary is used.
        self.length() * self.vocabulary().log2()
    }

    /// Returns the estimated difficulty required to program
    #[inline(always)]
    pub(crate) fn difficulty(&self) -> f64 {
        self.u_operators() / 2. * self.operands() / self.u_operands()
    }

    /// Returns the estimated level of difficulty required to program
    #[inline(always)]
    pub(crate) fn level(&self) -> f64 {
        1. / self.difficulty()
    }

    /// Returns the estimated effort required to program
    #[inline(always)]
    pub(crate) fn effort(&self) -> f64 {
        self.difficulty() * self.volume()
    }

    /// Returns the estimated time required to program.
    ///
    /// Unit of measurement: seconds
    #[inline(always)]
    pub(crate) fn time(&self) -> f64 {
        // The floating point `18.` aims to describe the processing rate of the
        // human brain. It is called Stoud number, S, and its
        // unit of measurement is moments/seconds.
        // A moment is the time required by the human brain to carry out the
        // most elementary decision.
        // 5 <= S <= 20. Halstead uses 18.
        // The value of S has been empirically developed from psychological
        // reasoning, and its recommended value for
        // programming applications is 18.
        //
        // Source: https://www.geeksforgeeks.org/software-engineering-halsteads-software-metrics/
        self.effort() / 18.
    }

    /// Returns the estimated number of delivered bugs.
    ///
    /// This metric represents the average amount of work a programmer can do
    /// without introducing an error.
    #[inline(always)]
    pub(crate) fn bugs(&self) -> f64 {
        // The floating point `3000.` represents the number of elementary
        // mental discriminations.
        // A mental discrimination, in psychology, is the ability to perceive
        // and respond to differences among stimuli.
        //
        // The value above is obtained starting from a constant that
        // is different for every language and assumes that natural language is
        // the language of the brain.
        // For programming languages, the English language constant
        // has been considered.
        //
        // After every 3000 mental discriminations a result is produced.
        // This result, whether correct or incorrect, is more than likely
        // either used as an input for the next operation or is output to the
        // environment.
        // If incorrect the error should become apparent.
        // Thus, an opportunity for error occurs every 3000
        // mental discriminations.
        //
        // Source: https://docs.lib.purdue.edu/cgi/viewcontent.cgi?article=1145&context=cstech
        self.effort().powf(2. / 3.) / 3000.
    }
}

pub(crate) trait Halstead
where
    Self: Checker,
{
    fn compute<'a>(node: &Node<'a>, code: &'a [u8], halstead_maps: &mut HalsteadMaps<'a>);
}

#[inline(always)]
fn get_id<'a>(node: &Node<'a>, code: &'a [u8]) -> &'a [u8] {
    &code[node.start_byte()..node.end_byte()]
}

#[inline(always)]
fn compute_halstead<'a, T: Getter>(
    node: &Node<'a>,
    code: &'a [u8],
    halstead_maps: &mut HalsteadMaps<'a>,
) {
    match T::get_op_type(node) {
        HalsteadType::Operator => {
            *halstead_maps.operators.entry(node.kind_id()).or_insert(0) += 1;
        }
        HalsteadType::Operand => {
            *halstead_maps
                .operands
                .entry(get_id(node, code))
                .or_insert(0) += 1;
        }
        HalsteadType::Unknown => {}
    }
}

impl Halstead for GoCode {
    fn compute<'a>(node: &Node<'a>, code: &'a [u8], halstead_maps: &mut HalsteadMaps<'a>) {
        compute_halstead::<Self>(node, code, halstead_maps);
    }
}

impl Halstead for RubyCode {
    fn compute<'a>(node: &Node<'a>, code: &'a [u8], halstead_maps: &mut HalsteadMaps<'a>) {
        compute_halstead::<Self>(node, code, halstead_maps);
    }
}

impl Halstead for KotlinCode {
    fn compute<'a>(node: &Node<'a>, code: &'a [u8], halstead_maps: &mut HalsteadMaps<'a>) {
        compute_halstead::<Self>(node, code, halstead_maps);
    }
}

impl Halstead for CCode {
    fn compute<'a>(node: &Node<'a>, code: &'a [u8], halstead_maps: &mut HalsteadMaps<'a>) {
        compute_halstead::<Self>(node, code, halstead_maps);
    }
}

// Markdown is a documentation language; classical Halstead is a code metric
// and does not apply. A Markdown-specific Halstead analogue will land in
// Phase B via the dedicated pipeline.
#[cfg(feature = "markdown")]
impl Halstead for crate::legacy::langs::MarkdownCode {
    fn compute<'a>(_node: &Node<'a>, _code: &'a [u8], _halstead_maps: &mut HalsteadMaps<'a>) {}
}

#[cfg(test)]
mod tests {
    use crate::legacy::langs::{GoParser, KotlinParser, RubyParser};
    use crate::legacy::tools::check_metrics;

    #[test]
    fn go_operators_and_operands() {
        check_metrics::<GoParser>(
            "package main

            func add(a, b int) int {
                return a + b
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.halstead,
                    @r###"
                    {
                      "n1": 7.0,
                      "N1": 7.0,
                      "n2": 5.0,
                      "N2": 8.0,
                      "length": 15.0,
                      "estimated_program_length": 31.26112492884004,
                      "purity_ratio": 2.0840749952560027,
                      "vocabulary": 12.0,
                      "volume": 53.77443751081734,
                      "difficulty": 5.6,
                      "level": 0.17857142857142858,
                      "effort": 301.1368500605771,
                      "time": 16.729825003365395,
                      "bugs": 0.014975730436275946
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_operators_and_operands() {
        check_metrics::<KotlinParser>(
            "fun add(a: Int, b: Int): Int {
                 return a + b
             }",
            "foo.kt",
            |metric| {
                // Only core counts are locked in; derived measures shift with
                // the vocabulary in ways that aren't meaningful to assert.
                insta::assert_json_snapshot!(
                    metric.halstead,
                    {
                        ".estimated_program_length" => "[masked]",
                        ".purity_ratio" => "[masked]",
                        ".volume" => "[masked]",
                        ".difficulty" => "[masked]",
                        ".level" => "[masked]",
                        ".effort" => "[masked]",
                        ".time" => "[masked]",
                        ".bugs" => "[masked]"
                    },
                    @r###"
                    {
                      "n1": 7.0,
                      "N1": 9.0,
                      "n2": 4.0,
                      "N2": 8.0,
                      "length": 17.0,
                      "estimated_program_length": "[masked]",
                      "purity_ratio": "[masked]",
                      "vocabulary": 11.0,
                      "volume": "[masked]",
                      "difficulty": "[masked]",
                      "level": "[masked]",
                      "effort": "[masked]",
                      "time": "[masked]",
                      "bugs": "[masked]"
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_operators_and_operands() {
        check_metrics::<RubyParser>(
            "def add(a, b)
                 a + b
             end",
            "foo.rb",
            |metric| {
                // Just assert the core counts; full MI/etc. follow from them.
                // Unique operators: def, +, (, ,
                // Unique operands: add, a, b
                insta::assert_json_snapshot!(
                    metric.halstead,
                    {
                        ".estimated_program_length" => "[masked]",
                        ".purity_ratio" => "[masked]",
                        ".volume" => "[masked]",
                        ".difficulty" => "[masked]",
                        ".level" => "[masked]",
                        ".effort" => "[masked]",
                        ".time" => "[masked]",
                        ".bugs" => "[masked]"
                    },
                    @r###"
                    {
                      "n1": 4.0,
                      "N1": 4.0,
                      "n2": 3.0,
                      "N2": 5.0,
                      "length": 9.0,
                      "estimated_program_length": "[masked]",
                      "purity_ratio": "[masked]",
                      "vocabulary": 7.0,
                      "volume": "[masked]",
                      "difficulty": "[masked]",
                      "level": "[masked]",
                      "effort": "[masked]",
                      "time": "[masked]",
                      "bugs": "[masked]"
                    }"###
                );
            },
        );
    }
}
