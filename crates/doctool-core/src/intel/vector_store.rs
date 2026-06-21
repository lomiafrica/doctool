use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct VectorStore {
    embeddings: HashMap<String, Vec<f32>>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_vector(&self, element_id: &str) -> Option<&Vec<f32>> {
        self.embeddings.get(element_id)
    }

    pub fn upsert(&mut self, element_id: impl Into<String>, vector: Vec<f32>) {
        self.embeddings.insert(element_id.into(), vector);
    }

    pub fn len(&self) -> usize {
        self.embeddings.len()
    }
}
