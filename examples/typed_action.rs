//! Use [`TypedAction`] to rewrite compute_dag.rs
//!
//! Only use Dag, execute a job. The graph is as follows:
//!
//!    ↱----------↴
//!    B -→ E --→ G
//!  ↗    ↗     ↗
//! A --→ C    /
//!  ↘    ↘  /
//!   D -→ F
//!
//! The final execution result is 272.

use std::sync::Arc;

use async_trait::async_trait;
use dagrs::{
    connection::{in_channel::TypedInChannels, out_channel::TypedOutChannels},
    node::typed_action::TypedAction,
    Content, DefaultNode, EnvVar, Graph, Node, NodeTable, Output,
};

const BASE: &str = "base";

struct Compute(usize);

#[async_trait]
impl TypedAction for Compute {
    type I = usize;
    type O = usize;

    async fn run(
        &self,
        mut in_channels: TypedInChannels<Self::I>,
        out_channels: TypedOutChannels<Self::O>,
        env: Arc<EnvVar>,
    ) -> Output {
        let base = env.get::<usize>(BASE).unwrap();
        let mut sum = self.0;

        // Collect all input values from input channels
        let inputs = in_channels
            .map(|result| {
                if let Ok(Some(value)) = result {
                    *value
                } else {
                    0
                }
            })
            .await;

        // Calculate the sum
        for input in inputs {
            sum += input * base;
        }

        // Broadcast the result to all output channels
        out_channels.broadcast(sum).await;

        Output::Out(Some(Content::new(sum)))
    }
}

fn main() {
    env_logger::init();

    let mut node_table = NodeTable::default();

    let a = DefaultNode::with_action("Compute A".to_string(), Compute(1), &mut node_table);
    let a_id = a.id();

    let b = DefaultNode::with_action("Compute B".to_string(), Compute(2), &mut node_table);
    let b_id = b.id();

    let mut c = DefaultNode::new("Compute C".to_string(), &mut node_table);
    c.set_action(Compute(4));
    let c_id = c.id();

    let mut d = DefaultNode::new("Compute D".to_string(), &mut node_table);
    d.set_action(Compute(8));
    let d_id = d.id();

    let e = DefaultNode::with_action("Compute E".to_string(), Compute(16), &mut node_table);
    let e_id = e.id();
    let f = DefaultNode::with_action("Compute F".to_string(), Compute(32), &mut node_table);
    let f_id = f.id();

    let g = DefaultNode::with_action("Compute G".to_string(), Compute(64), &mut node_table);
    let g_id = g.id();

    let mut graph = Graph::new();
    vec![a, b, c, d, e, f, g]
        .into_iter()
        .for_each(|node| graph.add_node(node));

    graph.add_edge(a_id, vec![b_id, c_id, d_id]);
    graph.add_edge(b_id, vec![e_id, g_id]);
    graph.add_edge(c_id, vec![e_id, f_id]);
    graph.add_edge(d_id, vec![f_id]);
    graph.add_edge(e_id, vec![g_id]);
    graph.add_edge(f_id, vec![g_id]);

    let mut env = EnvVar::new(node_table);
    env.set("base", 2usize);
    graph.set_env(env);

    match graph.start() {
        Ok(_) => {
            let res = graph
                .get_results::<usize>()
                .get(&g_id)
                .unwrap()
                .clone()
                .unwrap();
            // 验证执行结果
            assert_eq!(*res, 272)
        }
        Err(e) => {
            panic!("Graph execution failed: {:?}", e);
        }
    }
}
