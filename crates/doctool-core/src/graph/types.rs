use serde::{Deserialize, Serialize};

use crate::sources::mdx::MdxIndex;
use crate::sources::openapi::OpenApiIndex;
use crate::sources::sdk::SdkIndex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub fn build_knowledge_graph(
    openapi: &OpenApiIndex,
    mdx: &MdxIndex,
    sdk: Option<&SdkIndex>,
) -> KnowledgeGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for op in &openapi.operations {
        let id = format!("op:{} {}", op.method, op.path);
        nodes.push(GraphNode {
            id: id.clone(),
            kind: "operation".into(),
            label: format!("{} {}", op.method, op.path),
        });
    }

    for page in &mdx.pages {
        if page.locale != "en" {
            continue;
        }
        let id = format!("doc:{}", page.slug);
        nodes.push(GraphNode {
            id: id.clone(),
            kind: if page.method.is_some() {
                "api_doc".into()
            } else {
                "guide".into()
            },
            label: page.title.clone().unwrap_or_else(|| page.slug.clone()),
        });

        if let (Some(method), Some(path)) = (&page.method, &page.path) {
            let op_id = format!("op:{method} {path}");
            edges.push(GraphEdge {
                from: id.clone(),
                to: op_id,
                relation: "documents".into(),
            });
        }
    }

    if let Some(sdk) = sdk {
        for method in &sdk.methods {
            let id = format!("sdk:{}", method.qualified);
            nodes.push(GraphNode {
                id: id.clone(),
                kind: "sdk_method".into(),
                label: method.qualified.clone(),
            });
        }
    }

    KnowledgeGraph { nodes, edges }
}
