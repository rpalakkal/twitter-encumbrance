use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::twitter::{builder::TwitterClient, tweet::Tweet};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    role: String,
    content: Option<String>,
    name: Option<String>,
    function_call: Option<FunctionCall>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize, Debug)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    functions: Option<Vec<FunctionDefinition>>,
    function_call: Option<Value>, // Can be "auto" or a specific function name
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Serialize, Debug, Clone)]
struct FunctionDefinition {
    name: String,
    description: Option<String>,
    parameters: Value,
}

#[derive(Deserialize, Debug)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
}

struct Agent<'a> {
    api_key: String,
    client: Client,
    model: String,
    functions: Vec<FunctionDefinition>,
    system_prompt: Option<String>,
    twitter_client: TwitterClient<'a>,
}

impl<'a> Agent<'a> {
    fn new(functions: Vec<FunctionDefinition>, system_prompt: Option<String>, twitter_client: TwitterClient<'a>) -> Self {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        Agent {
            api_key,
            client: Client::new(),
            model: "gpt-4o".to_string(),
            functions,
            system_prompt,
            twitter_client,
        }
    }

    async fn call_openai_api(
        &self,
        messages: &Vec<Message>,
    ) -> eyre::Result<ChatCompletionResponse> {
        let request_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.clone(),
            functions: Some(self.functions.clone()),
            function_call: Some(serde_json::json!("auto")),
            max_tokens: Some(500),
            temperature: Some(0.5),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            eprintln!("Request failed with status: {}", response.status());
            let error_text = response.text().await?;
            eprintln!("Error body: {}", error_text);
            eyre::bail!("API request failed");
        }

        let completion_response: ChatCompletionResponse = response.json().await?;
        Ok(completion_response)
    }

    async fn handle_function_call(&self, function_call: &FunctionCall) -> eyre::Result<String> {
        match function_call.name.as_str() {
            "tweet_joke" => {
                let args: Value = serde_json::from_str(&function_call.arguments)?;
                let joke = args["joke"]
                    .as_str()
                    .ok_or_else(|| eyre::eyre!("Missing 'joke' field in arguments"))?;
                let result = tweet_joke(&self.twitter_client, joke).await;
                match result {
                    Ok(_) => Ok("Tweeted successfully".to_string()), // Return a success message
                    Err(e) => Err(e), // Propagate the error
                }
            }
            _ => eyre::bail!("Unknown function: {}", function_call.name),
        }
    }

    pub async fn run(&self, user_input: &str) -> eyre::Result<String> {
        let mut messages = Vec::new();

        // Add system prompt if provided
        if let Some(system_prompt) = &self.system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: Some(system_prompt.clone()),
                name: None,
                function_call: None,
            });
        }

        // Add user message
        messages.push(Message {
            role: "user".to_string(),
            content: Some(user_input.to_string()),
            name: None,
            function_call: None,
        });

        loop {
            let completion_response = self.call_openai_api(&messages).await?;

            let choice = &completion_response.choices[0];
            let message = &choice.message;

            if let Some(function_call) = &message.function_call {
                println!("Assistant is calling function: {}", function_call.name);
                println!("With arguments: {}", function_call.arguments);

                let function_response = self.handle_function_call(function_call).await?;

                messages.push(Message {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    function_call: Some(function_call.clone()),
                });

                messages.push(Message {
                    role: "function".to_string(),
                    content: Some(function_response),
                    name: Some(function_call.name.clone()),
                    function_call: None,
                });
            } else if let Some(content) = &message.content {
                return Ok(content.clone());
            } else {
                break;
            }
        }

        eyre::bail!("Failed to get a response from the assistant.")
    }
}

async fn tweet_joke<'a>(client: &TwitterClient<'a>, joke: &str) -> eyre::Result<()> {
    let tweet = Tweet::new(joke.to_string());
    match client.raw_tweet(tweet).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to tweet joke: {}", e); // Log the error
            Err(eyre::eyre!("Failed to tweet joke")) // Return a custom error message
        }
    }
}

pub async fn event_loop<'a>(twitter_client: TwitterClient<'a>) -> eyre::Result<()> {
    // Define the function(s) that the assistant can call
    let functions = vec![
        FunctionDefinition {
            name: "tweet_joke".to_string(),
            description: Some("Tweets a joke.".to_string()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "joke": { "type": "string", "description": "The joke to be tweeted." }
                },
            }),
        },
    ];

    let tweet_system_prompt =
        Some("You are great at coming up with surprising links and unhinged and deranged analogies between things and see the underlying similarity between seemingly different ideas. 
            And you are very snarky. 
            Your words should bring out people's sentiments and can go viral. 
            You need to make a tweet that is an unhinged joke about crypto.
            You think Ethereum L1 roadmap politics discussion is like supporting your local football team, you don't want it but everybody talks about it so you force yourself to read the ethresearch posts just to get invested in the characters.
            Don't use hashtags.".to_string());

    let agent = Agent::new(functions, tweet_system_prompt, twitter_client);

    let user_message = "make a tweet";

    loop {
        let response = agent.run(user_message).await?;
        println!("Assistant: {}", response);

        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
    Ok(())
}
