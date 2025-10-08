use crate::prelude::*;

#[derive(Default, Debug, Clone, Serialize)]
pub struct BarkController {
    pub text_variables: HashMap<VariableId, String>,
    pub embedding_variables: HashMap<VariableId, Vec<f32>>,
    pub prompts: HashMap<VariableId, Vec<BarkMessage>>,
    pub templates: HashMap<VariableId, Vec<MessageValue>>,
}

impl BarkController {
    pub fn new() -> Self {
        let mut text_variables = HashMap::new();
        text_variables.insert(
            VariableId::PreEmbed,
            "Represent this sentence for searching relevant passages: ".to_string(),
        );
        Self {
            text_variables,
            prompts: HashMap::new(),
            embedding_variables: HashMap::new(),
            templates: HashMap::new(),
        }
    }

    pub fn replace_template_variables(&self, line: &str) -> String {
        self.replace_template_variables_helper(line, Vec::new())
    }

    fn replace_template_variables_helper(&self, line: &str, visited: Vec<VariableId>) -> String {
        let mut result = String::new();
        let mut remaining = line;
        while remaining.contains("{{") && remaining.contains("}}") {
            let start = remaining.find("{{").unwrap();
            let end = remaining.find("}}").unwrap() + 2;
            result.push_str(&remaining[..start]);
            let key = &remaining[start + 2..end - 2];
            let variable_id = match key {
                "accumulator" => VariableId::Accumulator,
                "loop_value" => VariableId::LoopValue,
                "last_output" => VariableId::LastOutput,
                "pre_embed" => VariableId::PreEmbed,
                other => VariableId::User(other.to_string()),
            };

            // Check for loops
            if visited.contains(&variable_id) {
                result.push_str("<<WARNING:LOOP>>");
            } else {
                let replacement = self.text_variables.get(&variable_id);
                let replacement = replacement
                    .cloned()
                    .map(|s| {
                        let mut new_visited = visited.clone();
                        new_visited.push(variable_id);
                        self.replace_template_variables_helper(&s, new_visited)
                    })
                    .unwrap_or("".to_string());
                result.push_str(&replacement);
            }
            remaining = &remaining[end..];
        }
        result.push_str(remaining);
        result
    }

    pub fn template_from_str(&self, template_str: &str) -> Vec<MessageValue> {
        template_str
            .lines()
            .map(|line| self.replace_template_variables(line))
            .map(|line| {
                if line.starts_with("user:") {
                    MessageValue::User(line[5..].trim_start().to_string())
                } else if line.starts_with("system:") {
                    MessageValue::System(line[7..].trim_start().to_string())
                } else if line.starts_with("assistant:") {
                    MessageValue::Assistant(line[10..].trim_start().to_string())
                } else {
                    // eprintln!("Unknown message type in template: {}", line);
                    MessageValue::User(line.trim_start().to_string())
                }
            })
            .collect()
    }

    pub fn new_preloaded(
        preloaded_text: HashMap<String, String>,
        preloaded_templates: HashMap<String, Vec<MessageValue>>,
    ) -> Self {
        let mut text_variables = HashMap::new();
        for (key, value) in preloaded_text {
            text_variables.insert(VariableId::User(key), value);
        }
        let mut templates = HashMap::new();
        for (key, value) in preloaded_templates {
            templates.insert(VariableId::User(key), value);
        }
        Self {
            text_variables,
            prompts: HashMap::new(),
            templates,
            embedding_variables: HashMap::new(),
        }
    }

    pub fn get_prompt(&self, prompt: &PromptValue) -> Vec<BarkMessage> {
        match prompt {
            PromptValue::Variable(id) => self.prompts.get(id).cloned().unwrap_or(vec![]),
            PromptValue::Quick(s) => vec![user(s)],
            PromptValue::TemplateFile(text_value) => {
                let text = self.get_text(text_value);
                if text.ends_with(".json") {
                    // Assuming the file contains a JSON array of MessageValue
                    match std::fs::read_to_string(&text)
                        .map(|s| serde_json::from_str::<Vec<MessageValue>>(&s))
                    {
                        Ok(Ok(messages)) => self.get_prompt(&PromptValue::Chat(messages)),
                        Ok(Err(e)) => {
                            // eprintln!("Error parsing template file '{}': {}", text, e);
                            vec![]
                        }
                        Err(e) => {
                            // eprintln!("Error reading template file '{}': {}", text, e);
                            vec![]
                        }
                    }
                } else {
                    match std::fs::read_to_string(&text).map(|s| self.template_from_str(&s)) {
                        Ok(template) => self.get_prompt(&PromptValue::Chat(template)),
                        Err(e) => {
                            // eprintln!("Error reading template file '{}': {}", text, e);
                            vec![]
                        }
                    }
                }
            }
            PromptValue::Template(var) => {
                if let Some(template) = self.templates.get(var) {
                    self.get_prompt(&PromptValue::Chat(template.clone()))
                } else {
                    // eprintln!("Template not found: {:?}", var);
                    vec![]
                }
            }
            PromptValue::Chat(messages) => {
                let mut chat = vec![];
                for message in messages {
                    match message {
                        MessageValue::User(s) => chat.push(user(s)),
                        MessageValue::System(s) => chat.push(system(s)),
                        MessageValue::Assistant(s) => chat.push(assistant(s)),
                        MessageValue::UserVar(id) => chat.push(user(
                            &self.text_variables.get(id).cloned().unwrap_or_else(|| {
                                // eprintln!("User variable not found: {:?}", id);
                                String::new()
                            }),
                        )),
                        MessageValue::SystemVar(id) => chat.push(system(
                            &self.text_variables.get(id).cloned().unwrap_or_else(|| {
                                // eprintln!("User variable not found: {:?}", id);
                                String::new()
                            }),
                        )),
                        MessageValue::AssistantVar(id) => chat.push(assistant(
                            &self.text_variables.get(id).cloned().unwrap_or_else(|| {
                                // eprintln!("User variable not found: {:?}", id);
                                String::new()
                            }),
                        )),
                        MessageValue::UserVal(text) => chat.push(user(&self.get_text(text))),
                        MessageValue::SystemVal(text) => chat.push(system(&self.get_text(text))),
                        MessageValue::AssistantVal(text) => {
                            chat.push(assistant(&self.get_text(text)))
                        }
                        MessageValue::SubPrompt(id) => {
                            if let Some(sub_prompt) = self.prompts.get(id) {
                                chat.extend(sub_prompt.clone());
                            }
                        }
                        MessageValue::Template(id) => {
                            if let Some(template) = self.templates.get(id) {
                                let mut sub_prompt =
                                    self.get_prompt(&PromptValue::Chat(template.clone()));
                                chat.append(&mut sub_prompt);
                            } else {
                                // eprintln!("Template not found: {:?}", id);
                            }
                        }
                    }
                }
                chat
            }
            PromptValue::Joined(prompts) => {
                let mut chat = vec![];
                for prompt in prompts {
                    chat.extend(self.get_prompt(prompt));
                }
                chat
            }
        }
    }

    pub fn get_text(&self, text: &TextValue) -> String {
        match text {
            TextValue::Variable(id) => self.text_variables.get(id).cloned().unwrap_or_else(|| {
                // eprintln!("User variable not found: {:?}", id);
                String::new()
            }),
            TextValue::Default(id, default) => self
                .text_variables
                .get(id)
                .cloned()
                .unwrap_or_else(|| default.clone()),
            TextValue::Thoughts(id) => {
                let text = self.text_variables.get(id).cloned().unwrap_or_else(|| {
                    // eprintln!("User variable not found: {:?}", id);
                    String::new()
                });
                if text.contains("<think>") && text.contains("</think>") {
                    let start = text.find("<think>").unwrap() + 7;
                    let end = text.find("</think>").unwrap();
                    return text[start..end].to_string();
                } else {
                    return String::new();
                }
            }
            TextValue::WithoutThoughts(id) => {
                let text = self.text_variables.get(id).cloned().unwrap_or_else(|| {
                    // eprintln!("User variable not found: {:?}", id);
                    String::new()
                });
                strip_thoughts(&text)
            }
            TextValue::Simple(s) => s.clone(),
            TextValue::Multi(texts) => texts.iter().map(|t| self.get_text(t)).collect(),
            TextValue::Structured(s) => {
                let mut output = HashMap::new();
                for (key, value) in s {
                    output.insert(key.clone(), self.get_text(value));
                }
                serde_json::to_string(&output).unwrap()
            }
        }
    }

    pub fn text_matches(&self, text: &TextValue, matcher: &TextMatcher) -> bool {
        match matcher {
            TextMatcher::Exact(value) => self
                .get_text(text)
                .trim()
                .eq_ignore_ascii_case(self.get_text(value).trim()),
            TextMatcher::Contains(value) => self.get_text(text).contains(&self.get_text(value)),
            TextMatcher::StartsWith(value) => {
                self.get_text(text).starts_with(&self.get_text(value))
            }
            TextMatcher::EndsWith(value) => self.get_text(text).ends_with(&self.get_text(value)),
            // TextMatcher::Regex(value) => {
            //     let re = regex::Regex::new(&self.get_text(value)).unwrap();
            //     re.is_match(&self.get_text(text))
            // }
            TextMatcher::Not(inner) => !self.text_matches(text, inner),
            TextMatcher::Any(matchers) => matchers.iter().any(|m| self.text_matches(text, m)),
            TextMatcher::All(matchers) => matchers.iter().all(|m| self.text_matches(text, m)),
        }
    }

    pub fn start_prompt(&mut self, id: VariableId, messages: PromptValue) {
        let prompt = self.get_prompt(&messages);
        self.prompts.insert(id, prompt);
    }

    pub fn extend_prompt(&mut self, id: VariableId, messages: PromptValue) {
        let prompt = self.get_prompt(&messages);
        self.prompts
            .entry(id)
            .or_insert_with(Vec::new)
            .extend(prompt);
    }

    pub fn replace_system_prompt(&mut self, id: VariableId, messages: PromptValue) {
        let mut prompt = self.get_prompt(&messages);
        if let Some(existing) = self.prompts.get_mut(&id) {
            existing.retain(|msg| msg.role != BarkRole::System);
            prompt.extend(existing.clone());
            self.prompts.insert(id, prompt);
        } else {
            self.prompts.insert(id, prompt);
        }
    }
}

pub fn strip_thoughts(text: &String) -> String {
    if text.contains("<think>") && text.contains("</think>") {
        let start = text.find("<think>").unwrap();
        let end = text.find("</think>").unwrap();
        return format!("{}{}", &text[..start], &text[end + 8..])
            .trim()
            .to_string();
    } else {
        return text.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_thoughts() {
        let input = "<think>this is a thought</think>Hello World".to_string();
        let expected = "Hello World".to_string();
        assert_eq!(strip_thoughts(&input), expected);
    }

    #[test]
    fn test_strip_thoughts_no_thoughts() {
        let input = "Hello World".to_string();
        let expected = "Hello World".to_string();
        assert_eq!(strip_thoughts(&input), expected);
    }

    #[test]
    fn test_replace_single_template() {
        let mut controller = BarkController::new();
        let id = VariableId::LastOutput;
        controller
            .text_variables
            .insert(id, "Hello world".to_string());
        let line = "{{last_output}}";
        let replaced = controller.replace_template_variables(line);
        assert_eq!(replaced, "Hello world");
    }

    #[test]
    fn test_replace_multiple_templates() {
        let mut controller = BarkController::new();
        let id1 = VariableId::LastOutput;
        let id2 = VariableId::Accumulator;
        controller
            .text_variables
            .insert(id1, "Hello world".to_string());
        controller
            .text_variables
            .insert(id2, "Goodbye world".to_string());

        let line = "{{last_output}} and {{accumulator}}";
        let replaced = controller.replace_template_variables(line);
        assert_eq!(replaced, "Hello world and Goodbye world");
    }

    #[test]
    fn test_replace_templates_recursive() {
        let mut controller = BarkController::new();
        let id1 = VariableId::LastOutput;
        let id2 = VariableId::Accumulator;
        controller
            .text_variables
            .insert(id1, "{{accumulator}}".to_string());
        controller
            .text_variables
            .insert(id2, "Goodbye world".to_string());
        let line = "{{last_output}} and {{accumulator}}!";
        let replaced = controller.replace_template_variables(line);
        assert_eq!(replaced, "Goodbye world and Goodbye world!");
    }

    #[test]
    fn test_replace_templates_loop_detection() {
        let mut controller = BarkController::new();
        let id1 = VariableId::LastOutput;
        let id2 = VariableId::Accumulator;
        // Create a loop: last_output -> accumulator -> last_output
        controller
            .text_variables
            .insert(id1, "{{accumulator}}".to_string());
        controller
            .text_variables
            .insert(id2, "{{last_output}}".to_string());
        let line = "{{last_output}}";
        let replaced = controller.replace_template_variables(line);
        assert_eq!(replaced, "<<WARNING:LOOP>>");
    }

    #[test]
    fn test_replace_templates_self_reference() {
        let mut controller = BarkController::new();
        let id = VariableId::LastOutput;
        // Create self-reference loop
        controller
            .text_variables
            .insert(id, "I am {{last_output}}".to_string());
        let line = "{{last_output}}";
        let replaced = controller.replace_template_variables(line);
        assert_eq!(replaced, "I am <<WARNING:LOOP>>");
    }
}
