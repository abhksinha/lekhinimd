use lekhni_index::doc_table::{DocRecord, DocTable};
use lekhni_index::link_graph::LinkGraph;
use lekhni_index::trigram::TrigramIndex;

#[test]
fn test_trigram_search_query() {
    let mut index = TrigramIndex::new();

    let doc1 = b"Rust is a systems programming language focused on safety.";
    let doc2 = b"Python is an interpreted high-level general-purpose language.";
    let doc3 = b"Systems architecture requires cache-conscious programming.";

    index.index_doc(1, doc1);
    index.index_doc(2, doc2);
    index.index_doc(3, doc3);

    // Query for "systems"
    let candidates = index.query_candidates(b"systems");
    assert!(candidates.contains(&1));
    assert!(candidates.contains(&3));
    assert!(!candidates.contains(&2));

    // Query for "python"
    let candidates = index.query_candidates(b"python");
    assert_eq!(candidates, vec![2]);
}

#[test]
fn test_link_graph_bidirectional() {
    let mut graph = LinkGraph::new();
    // Doc 1 links to Doc 2 and Doc 3
    graph.add_link(1, 2);
    graph.add_link(1, 3);
    // Doc 2 links to Doc 3
    graph.add_link(2, 3);
    graph.finalize();

    // Forward links from Doc 1
    assert_eq!(graph.forward_links(1), vec![2, 3]);

    // Backlinks pointing to Doc 3
    assert_eq!(graph.backlinks(3), vec![1, 2]);
}

#[test]
fn test_doc_table() {
    let mut table = DocTable::new();
    table.insert(DocRecord {
        id: 1,
        path_hash: 0x1234,
        mtime: 100,
        size: 500,
        title: "Introduction".into(),
        path: "intro.md".into(),
    });

    assert_eq!(table.len(), 1);
    let record = table.get_by_id(1).unwrap();
    assert_eq!(record.title, "Introduction");
}
