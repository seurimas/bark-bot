use crate::prelude::*;

pub mod checker;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputOutputPair {
    pub input: Vec<Vec<u8>>,
    pub output: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingData {
    pub test: Vec<InputOutputPair>,
    pub train: Vec<InputOutputPair>,
}

impl InputOutputPair {
    pub fn describe(&self) -> String {
        format!("{}{}", self.describe_input(), self.describe_output(),)
    }

    pub fn describe_input(&self) -> String {
        format!(
            "The input looks like this:\n```\n{}\n```\n\n",
            self.input
                .iter()
                .map(|x| x
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", "))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    pub fn describe_output(&self) -> String {
        format!(
            "The output looks like this:\n```\n{}\n```\n\n",
            self.output
                .iter()
                .map(|x| x
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", "))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}
