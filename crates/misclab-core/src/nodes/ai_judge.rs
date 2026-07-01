use super::prelude::*;

/// Ask the configured LLM to analyze/judge the input and return a formatted answer.
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?;
        let system = pstr(
            params,
            "system",
            "你是一名 CTF misc 解题助手，回答简洁、只给结论。",
        );
        let instruction = pstr(params, "instruction", "分析以下内容并给出结果：");
        let format = pstr(params, "format", "");

        let mut user = format!("{instruction}\n\n{input}");
        if !format.trim().is_empty() {
            user.push_str(&format!(
                "\n\n请严格按以下格式输出（只输出结果本身，不要额外说明）：\n{format}"
            ));
        }

        let answer = ai::chat(&ctx.env.ai.llm, system, &user)?;
        Ok(out_text(answer))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "ai_judge",
            AI,
            "AI 判断",
            EMERALD,
            vec![t_in()],
            vec![req("text", "回答", PortType::Text)],
            vec![
                ParamSpec::text(
                    "system",
                    "系统提示",
                    "你是一名 CTF misc 解题助手，回答简洁、只给结论。",
                    true,
                ),
                ParamSpec::text("instruction", "指令", "分析以下内容并给出结果：", true),
                ParamSpec::text("format", "输出格式(可选)", "", true),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
