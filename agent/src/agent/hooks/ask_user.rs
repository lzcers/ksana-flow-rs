use crate::agent::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleFlow, LifeCycleInterrupt, LifeCycleResult},
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct AskUserHook {}

impl AskUserHook {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl LifeCycleHook for AskUserHook {
    fn name(&self) -> HookName {
        "ask_user"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn on(&self, stage: &LifeCycle) -> bool {
        matches!(stage, LifeCycle::AfterCallModel)
    }

    async fn handle(&mut self, ctx: &mut LifeCycleContext) -> LifeCycleFlow {
        if let Some(tools_call) = ctx.frame.get_tools_call() {
            if let Some(ask_user_tool) =
                tools_call.iter().find(|tool| tool.get_name() == "ask_user")
            {
                let args = ask_user_tool.get_arguments();
                let question = args
                    .get("question")
                    .and_then(|param| param.as_str())
                    .unwrap_or("请输入")
                    .to_string();

                // 使用中断机制来处理 AskUser
                return LifeCycleFlow::Break(LifeCycleInterrupt::ask_user(question));
            }
        }

        LifeCycleFlow::Continue(LifeCycleResult::None)
    }
}
