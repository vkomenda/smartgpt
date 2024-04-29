use async_trait::async_trait;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fmt::{Debug, Display},
};

use crate::plugins::http_client::HttpClientMethod;
use crate::{error::OpaqueError, http_client::HttpClientState};
use crate::{MemorySystem, ScriptValue, LLM};

#[derive(Debug, Clone)]
pub struct ToolStateNoInvoke(pub String, pub String);

impl<'a> Display for ToolStateNoInvoke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the '{}' plugin's data does not have '{}' invocation.",
            self.0, self.1
        )
    }
}

impl<'a> Error for ToolStateNoInvoke {}

#[derive(Debug, Clone)]
pub struct CommandNoArgError<'a>(pub &'a str, pub &'a str);

impl<'a> Display for CommandNoArgError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the '{}' tool did not receive the '{}' argument.",
            self.0, self.1
        )
    }
}

impl<'a> Error for CommandNoArgError<'a> {}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum StatefulTool {
    HttpClient,
    Google,
    NewsApi,
    Wolfram,
    AlphaVantage,
}

#[async_trait]
pub trait ToolState: Send + Sync {
    type Method;

    async fn call_method(
        &mut self,
        method: Self::Method,
        args: Value,
    ) -> Result<Value, OpaqueError>;
}

pub enum ToolMethod {
    HttpClientMethod(HttpClientMethod),
    GoogleMethod(GoogleMethod),
}

pub struct ToolStateStore(pub BTreeMap<StatefulTool, Box<dyn ToolState>>);

impl ToolStateStore {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn call_method(&mut self, method: ToolMethod, args: Value) -> Result<Value, OpaqueError> {
        match method {
            ToolMethod::HttpClientMethod(http_client_method) => {
                let tool_state: HttpClientState = self.get_state(Tool::HttpClient);
                tool_state.call_method(http_client_method, args)
            }

            ToolMethod::GoogleMethod(google_method) => {
                let tool_state: GooleState = self.get_state(Tool::Google);
                tool_state.call_method(google_method, args)
            } //
              // TODO
              //
        }
    }

    pub fn get_state(
        &mut self,
        module: StatefulTool,
    ) -> Result<&mut Box<dyn ToolState>, OpaqueError> {
        self.0
            .get_mut(&module)
            .ok_or(Box::new(NoToolStateError(module)))
    }
}

pub struct EndGoals {
    pub end_goal: usize,
    pub end_goals: Vec<String>,
}

impl EndGoals {
    pub fn get(&self) -> String {
        self.end_goals[self.end_goal].clone()
    }
}

pub struct AgentInfo {
    pub llm: LLM,
    pub observations: Box<dyn MemorySystem>,
    pub reflections: Box<dyn MemorySystem>,
}

pub struct Agents {
    pub static_agent: AgentInfo,
    pub planner: AgentInfo,
    pub dynamic: AgentInfo,
    pub fast: AgentInfo,
}

impl Agents {
    pub fn same(
        init: impl Fn() -> Result<AgentInfo, Box<dyn Error>>,
    ) -> Result<Agents, Box<dyn Error>> {
        Ok(Agents {
            static_agent: init()?,
            planner: init()?,
            dynamic: init()?,
            fast: init()?,
        })
    }
}

pub struct CommandContext {
    pub module_state_store: ToolStateStore,
    pub agents: Agents,
    pub plugins: Vec<Tool>,
    pub disabled_tools: Vec<String>,
    pub assets: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct NoToolStateError(pub StatefulTool);

impl Display for NoToolStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not find plugin data for plugin \"{}\"", self.0)
    }
}

impl Error for NoToolStateError {}

pub async fn invoke(data: &mut Box<dyn ModuleState>, method: M) -> Result<O, Box<dyn Error>> {
    let value = data.apply(name).await?;
    Ok(value)
}

pub enum CommandResult {
    ScriptValue(ScriptValue),
    Text(String),
}

#[async_trait]
pub trait CommandImpl: Send + Sync {
    async fn invoke(
        &self,
        ctx: &mut CommandContext,
        args: ScriptValue,
    ) -> Result<CommandResult, Box<dyn Error>>;

    fn box_clone(&self) -> Box<dyn CommandImpl>;
}

#[async_trait]
pub trait ToolCycle: Send + Sync {
    async fn create_context(
        &self,
        context: &mut CommandContext,
        previous_prompt: Option<&str>,
    ) -> Result<Option<String>, Box<dyn Error>>;
    fn create_data(&self, value: Value) -> Option<Box<dyn ToolState>>;
}

pub struct EmptyCycle;

#[async_trait]
impl ToolCycle for EmptyCycle {
    async fn create_context(
        &self,
        _context: &mut CommandContext,
        _previous_prompt: Option<&str>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        Ok(None)
    }

    fn create_data(&self, _: Value) -> Option<Box<dyn ToolState>> {
        None
    }
}

#[derive(Clone)]
pub struct ToolFeatureArgument {
    pub name: String,
    pub example: String,
}

impl ToolFeatureArgument {
    pub fn new(name: &str, example: &str) -> Self {
        Self {
            name: name.to_string(),
            example: example.to_string(),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ToolFeatureType {
    Resource,
    Action { needs_permission: bool },
}

pub struct ToolFeature {
    pub name: String,
    pub purpose: String,
    pub args: Vec<ToolArgument>,
    pub feature_type: ToolFeatureType,
    pub run: Box<dyn CommandImpl>,
}

impl ToolFeature {
    pub fn box_clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            purpose: self.purpose.clone(),
            args: self.args.clone(),
            feature_type: self.feature_type,
            run: Box::new(self.run),
        }
    }
}

pub struct Tool {
    pub id: StatefulTool,
    pub cycle: Box<dyn ToolCycle>,
    pub dependencies: Vec<StatefulTool>,
    pub features: Vec<ToolFeature>,
}
