use super::{BarkChat, BarkResponse, BarkRole};

impl From<openai_api_rust::completions::Completion> for BarkResponse {
    fn from(response: openai_api_rust::completions::Completion) -> Self {
        Self {
            choices: response
                .choices
                .into_iter()
                .enumerate()
                .map(|(idx, c)| super::Choice {
                    index: idx,
                    value: c.message.unwrap().content,
                })
                .collect(),
            usage: response.usage.total_tokens,
        }
    }
}

impl From<BarkChat> for openai_api_rust::chat::ChatBody {
    fn from(chat: BarkChat) -> Self {
        let mut combined: Vec<openai_api_rust::Message> = vec![];
        let mut combined_message = openai_api_rust::Message {
            role: openai_api_rust::Role::User,
            content: "".to_string(),
        };
        for message in chat.messages {
            if let Some(top) = combined.last_mut() {
                if matches!(top.role, openai_api_rust::Role::User)
                    == matches!(message.role, BarkRole::User)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(top.role, openai_api_rust::Role::Assistant)
                    == matches!(message.role, BarkRole::Assistant)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(top.role, openai_api_rust::Role::System)
                    == matches!(message.role, BarkRole::System)
                {
                    top.content.push_str(&message.content);
                    continue;
                }
            }
            combined_message.role = match message.role {
                BarkRole::User => openai_api_rust::Role::User,
                BarkRole::Assistant => openai_api_rust::Role::Assistant,
                BarkRole::System => openai_api_rust::Role::System,
            };
            combined.push(combined_message);
            combined_message = openai_api_rust::Message {
                role: openai_api_rust::Role::User,
                content: "".to_string(),
            };
        }
        openai_api_rust::chat::ChatBody {
            frequency_penalty: None,
            logit_bias: None,
            max_tokens: Some(4096),
            messages: combined,
            model: chat.model,
            n: None,
            presence_penalty: None,
            stop: None,
            stream: None,
            temperature: None,
            top_p: None,
            user: None,
        }
    }
}
