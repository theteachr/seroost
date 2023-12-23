use std::collections::HashMap;
use std::path::{PathBuf, Path};
use serde::{Deserialize, Serialize};
use super::lexer::Lexer;
use std::time::SystemTime;

type DocFreq = HashMap<String, usize>;
type TermFreq = HashMap<String, usize>;
#[derive(Deserialize, Serialize)]
pub struct Doc {
    tf: TermFreq,
    count: usize,
    // TODO: make sure that the serde serialization of SystemTime also work on other platforms
    last_modified: SystemTime,
}
type Docs = HashMap<PathBuf, Doc>;

#[derive(Default, Deserialize, Serialize)]
pub struct Model {
    pub docs: Docs,
    pub df: DocFreq,
}

impl Model {
    fn remove_document(&mut self, file_path: &Path) {
        self.docs.remove(file_path).into_iter().for_each(|mut doc| {
            doc.tf
                .keys()
                .filter_map(|term| self.df.get_mut(term))
                .for_each(|f| *f -= 1)
        })
    }

    pub fn requires_reindexing(&mut self, file_path: &Path, last_modified: SystemTime) -> bool {
        self.docs
            .get(file_path)
            .filter(|doc| doc.last_modified < last_modified)
            .is_some()
    }

    pub fn search_query(&self, query: &[char]) -> Vec<(PathBuf, f32)> {
        let mut result = Vec::new();
        let tokens = Lexer::new(&query).collect::<Vec<_>>();
        for (path, doc) in &self.docs {
            let mut rank = 0f32;
            for token in &tokens {
                rank += compute_tf(token, doc) * compute_idf(&token, self.docs.len(), &self.df);
            }
            // TODO: investigate the sources of NaN
            if !rank.is_nan() {
                result.push((path.clone(), rank));
            }
        }
        result.sort_by(|(_, rank1), (_, rank2)| rank1.partial_cmp(rank2).expect(&format!("{rank1} and {rank2} are not comparable")));
        result.reverse();
        result
    }

    pub fn add_document(&mut self, file_path: PathBuf, last_modified: SystemTime, content: &[char]) {
        self.remove_document(&file_path);

        let mut tf = TermFreq::new();

        let count = Lexer::new(content)
            .into_iter()
            .map(|term| {
                *tf.entry(&t).or_insert(0) += 1;
            })
            .count();

        tf.keys().for_each(|term| {
            *self.df.entry(term).or_insert(0) += 1;
        });

        self.docs.insert(file_path, Doc {count, tf, last_modified});
    }
}

fn compute_tf(t: &str, doc: &Doc) -> f32 {
    let n = doc.count as f32;
    let m = doc.tf.get(t).cloned().unwrap_or(0) as f32;
    m / n
}

fn compute_idf(t: &str, n: usize, df: &DocFreq) -> f32 {
    let n = n as f32;
    let m = df.get(t).cloned().unwrap_or(1) as f32;
    (n / m).log10()
}
