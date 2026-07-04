//! 字节序交换：把字节按 2/4/8 一组翻转（大小端互换），或整体反转。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let gs = match pstr(p, "groupSize", "4") {
            "2" => 2,
            "8" => 8,
            "整个" => data.len().max(1),
            _ => 4,
        };
        let mut out = Vec::with_capacity(data.len());
        for chunk in data.chunks(gs) {
            out.extend(chunk.iter().rev());
        }
        let mut m = PortMap::new();
        m.insert("hex".into(), PortValue::Text(hex::encode(&out)));
        m.insert("bytes".into(), PortValue::Bytes(Arc::from(out.into_boxed_slice())));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "endian_swap",
            UTIL,
            "字节序交换",
            SLATE,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("bytes", "字节", PortType::Bytes),
                opt("hex", "Hex", PortType::Text),
            ],
            vec![ParamSpec::select("groupSize", "分组", &["2", "4", "8", "整个"], "4")],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    #[test]
    fn swaps_in_groups_of_four() {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(vec![1u8, 2, 3, 4, 5, 6, 7, 8].into_boxed_slice())));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "endian_swap",
            &i,
            &serde_json::json!({ "groupSize": "4" }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("bytes") {
            Some(PortValue::Bytes(b)) => assert_eq!(&b[..], &[4, 3, 2, 1, 8, 7, 6, 5]),
            o => panic!("{o:?}"),
        }
    }
}
