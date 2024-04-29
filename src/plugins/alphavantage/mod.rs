mod types;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt::Display;

use crate::{
    invoke, BrowseRequest, CommandContext, CommandImpl, CommandResult, Plugin, PluginCycle,
    PluginData, ScriptValue, Tool, ToolArgument, ToolType,
};

pub use types::*;

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct GlobalQuoteArgs {
//     symbol: String,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct NewsSentimentArgs {
//     tickers: String,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(tag = "function", content = "args")]
// #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
// pub enum AlphaVantageRequest {
//     GlobalQuote(GlobalQuoteArgs),
//     NewsSentiment(NewsSentimentArgs),
// }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AlphaVantageRequest {
    symbol: String,
}

#[derive(Serialize, Deserialize)]
pub struct AlphaVantageData {
    #[serde(rename = "api key")]
    pub api_key: String,
}

#[async_trait]
impl PluginData for AlphaVantageData {
    // TODO: refactor name into enum types perhaps
    async fn apply(&mut self, _name: &str) -> Result<Value, Box<dyn Error>> {
        Ok(self.api_key.clone().into())
    }
}

pub async fn ask_alphavantage(
    ctx: &mut CommandContext,
    req: &AlphaVantageRequest,
) -> Result<String, Box<dyn Error>> {
    let alphavantage_data = ctx.plugin_data.get_data("AlphaVantage")?;
    let api_key: String =
        serde_json::from_value(invoke(alphavantage_data, "get api key", true).await?)?;
    let api_key: &str = &api_key;

    // let params = match req {
    //     AlphaVantageRequest::GlobalQuote(args) => [
    //         ("apikey", api_key),
    //         ("function", "GLOBAL_QUOTE"),
    //         ("symbol", &args.symbol),
    //     ],
    //     AlphaVantageRequest::NewsSentiment(args) => [
    //         ("apikey", api_key),
    //         ("function", "NEWS_SENTIMENT"),
    //         ("tickers", &args.tickers),
    //     ],
    // };
    let params = [
        ("apikey", api_key),
        ("function", "GLOBAL_QUOTE"),
        ("symbol", &req.symbol),
    ];

    let browse_info = ctx.plugin_data.get_data("Browse")?;
    let json: QuoteObject = serde_json::from_value(
        invoke(
            browse_info,
            "browse",
            BrowseRequest {
                url: "https://www.alphavantage.co/query".to_string(),
                params: params
                    .iter()
                    .map(|el| (el.0.to_string(), el.1.to_string()))
                    .collect::<Vec<_>>(),
            },
        )
        .await?,
    )?;

    // let resp = match req {
    //     AlphaVantageRequest::GlobalQuote(_) => {
    //         let json: GlobalQuote = serde_json::from_str(&json)?;

    //         json.price
    //     }
    //     AlphaVantageRequest::NewsSentiment(_) => {
    //         let json: AlphaVantageNews = serde_json::from_str(&json)?;

    //         json.feed
    //             .iter()
    //             .flat_map(|item| {
    //                 vec![
    //                     format!("# [{}]({})", item.title, item.url),
    //                     item.summary.clone(),
    //                 ]
    //             })
    //             .collect::<Vec<_>>()
    //             .join("\n")
    //     }
    // };

    //    let json: QuoteObject = serde_json::from_str(&json)?;
    let resp = json.global_quote.price;

    Ok(resp)
}

pub async fn alphavantage(
    ctx: &mut CommandContext,
    args: ScriptValue,
) -> Result<CommandResult, Box<dyn Error>> {
    let req: AlphaVantageRequest = args.parse()?;
    let response = ask_alphavantage(ctx, &req).await?;

    Ok(CommandResult::Text(response))
}

pub struct AlphaVantageImpl;

#[async_trait]
impl CommandImpl for AlphaVantageImpl {
    async fn invoke(
        &self,
        ctx: &mut CommandContext,
        args: ScriptValue,
    ) -> Result<CommandResult, Box<dyn Error>> {
        alphavantage(ctx, args).await
    }

    fn box_clone(&self) -> Box<dyn CommandImpl> {
        Box::new(Self)
    }
}

pub struct AlphaVantageCycle;

#[async_trait]
impl PluginCycle for AlphaVantageCycle {
    async fn create_context(
        &self,
        _context: &mut CommandContext,
        _previous_prompt: Option<&str>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        Ok(None)
    }

    fn create_data(&self, value: Value) -> Option<Box<dyn PluginData>> {
        let data: AlphaVantageData = serde_json::from_value(value).ok()?;
        Some(Box::new(data))
    }
}

pub fn create_alphavantage() -> Plugin {
    Plugin {
        name: "AlphaVantage".to_string(),
        dependencies: vec!["Browse".to_string()],
        cycle: Box::new(AlphaVantageCycle),
        tools: vec![
            Tool {
                name: "alphavantage_quote".to_string(),
                purpose: "Read ticker prices from AlphaVantage.".to_string(),
                args: vec![
                    ToolArgument::new("function", "GLOBAL_QUOTE"),
                    ToolArgument::new("symbol", "AAPL"),
                ],
                run: Box::new(AlphaVantageImpl),
                tool_type: ToolType::Resource,
            },
            // Tool {
            //     name: "alphavantage_news".to_string(),
            //     purpose: "Read market news about tickers from AlphaVantage.".to_string(),
            //     args: vec![
            //         ToolArgument::new("function", "NEWS_SENTIMENT"),
            //         ToolArgument::new(
            //             "args",
            //             r#""args": { "tickers": "AAPL,CRYPTO:BTC,FOREX:USD" }"#,
            //         ),
            //     ],
            //     run: Box::new(AlphaVantageImpl),
            //     tool_type: ToolType::Resource,
            // },
        ],
    }
}
