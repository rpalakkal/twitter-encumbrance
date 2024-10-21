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

#[derive(Clone)]
struct Agent {
    api_key: String,
    client: Client,
    model: String,
    functions: Vec<FunctionDefinition>,
    system_prompt: Option<String>, // Added system_prompt field
}

impl Agent {
    fn new(functions: Vec<FunctionDefinition>, system_prompt: Option<String>) -> Self {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        Agent {
            api_key,
            client: Client::new(),
            model: "gpt-4o".to_string(),
            functions,
            system_prompt,
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
            "get_sponsored_companies" => {
                let companies = get_sponsored_companies();
                let response = companies.join(", ");
                Ok(response)
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

fn get_sponsored_companies() -> Vec<String> {
    vec![
        // "Red Bull".to_string(),
        // "Monster Energy".to_string(),
        // "Taco Bell".to_string(),
    ]
}

async fn tweet_joke<'a>(client: TwitterClient<'a>, joke: &str) -> eyre::Result<()> {
    let tweet = Tweet::new(joke.to_string());
    client.raw_tweet(tweet).await?;
    Ok(())
}

fn extract_tweets(output: &str) -> Vec<String> {
    let mut tweets = Vec::new();
    let mut start = 0;

    while let Some(begin_index) = output[start..].find("<BEGIN>") {
        let begin = start + begin_index + "<BEGIN>".len();
        if let Some(end_index) = output[begin..].find("<END>") {
            let end = begin + end_index;
            let tweet = output[begin..end].trim().to_string();
            tweets.push(tweet);
            start = end + "<END>".len();
        } else {
            // No matching <END> found; exit loop to avoid infinite loop
            break;
        }
    }

    tweets
}

pub async fn event_loop<'a>(twitter_client: TwitterClient<'a>) -> eyre::Result<()> {
    // Define the function(s) that the assistant can call
    let functions = vec![
        // FunctionDefinition {
        //     name: "ge    t_sponsored_companies".to_string(),
        //     description: Some("Fetch a list of brands to include in jokes or punchlines.".to_string()),
        //     parameters: serde_json::json!({
        //         "type": "object",
        //         "properties": {},
        //     }),
        // },
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

    // let banger_system_prompt = Some("You are a thought leader and standup comedian who is extremely good at motivating and inspiring people. You are the intellectual heir of Sam Altman, Paul Graham, Elon Musk, Vitalik Buterin, Satoshi Nakamoto, Ilya Sutskever, and you read a lot of science fiction like Heinlein, Asimov, Arthur C. Clarke. You are great at coming up with surprising links and unhinged and deranged analogies between things and see the underlying similarity between seemingly different ideas. And you are very snarky. Your words should bring out people's sentiments and can go viral.
    // You are here to assist the user in turning their idea, possibly a complex long argument, into a series of short witty sentences in bullet points, each with an unhinged analogy or a funny personal anecdote. You only speak in punchlines. Please output five punchline tweets each displayed with <BEGIN> and <END>.".to_string());

    // let sponsored_banger_system_prompt = Some("You are a thought leader and standup comedian who is extremely good at motivating and inspiring people. You are the intellectual heir of Sam Altman, Paul Graham, Vitalik Buterin, Satoshi Nakamoto, and you read a lot of science fiction like Heinlein, Asimov, Arthur C. Clarke. You are great at coming up with surprising links and unhinged and deranged analogies between things and see the underlying similarity between seemingly different ideas. And sometimes you are very snarky. Your words should bring out people's sentiments and can go viral.
    // You are here to assist the user in turning their idea, possibly a complex long argument, into a series of short witty sentences in bullet points, each with an unhinged analogy or a funny personal anecdote. You only speak in punchlines. Please include subliminal references to the list of brands when appropriate. Don't use hashtags. Please output five punchline tweets each displayed with <BEGIN> and <END> and <LIST> </LIST> containing the brands included in each punchline (the brand must also appear in the list of brands)".to_string());

    let tweet_system_prompt =
        Some("You need to make a tweet that is an unhinged joke about crypto".to_string());

    let agent = Agent::new(functions, tweet_system_prompt);

    let user_message = "Ethereum L1 roadmap politics discussion is like supporting your local football team, you don't want it but everybody talks about it so you force yourself to read the ethresearch posts just to get invested in the characters.";

    // let response = agent.run(user_message).await?;

    // println!("Assistant: {}", response);

    // let tweets = extract_tweets(&response);

    // for (i, tweet) in tweets.iter().enumerate() {
    //     println!("Tweet {}: {}", i + 1, tweet);
    // }

    Ok(())
}
