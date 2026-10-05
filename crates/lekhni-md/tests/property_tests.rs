use lekhni_md::block::BlockKind;
use lekhni_md::parser::MarkdownParser;

#[test]
fn test_incremental_parse_equals_full_parse_property() {
    let mut initial_lines = vec![
        "# Heading 1".to_string(),
        "Line 1 content".to_string(),
        "Line 2 content".to_string(),
        "```rust".to_string(),
        "fn test() {}".to_string(),
        "```".to_string(),
        "> Blockquote line".to_string(),
        "- List item 1".to_string(),
        "- List item 2".to_string(),
    ];

    let full_doc = initial_lines.join("\n") + "\n";
    let mut full_parser = MarkdownParser::new();
    full_parser.parse_full(full_doc.as_bytes());

    let mut inc_parser = MarkdownParser::new();
    inc_parser.checkpoints.interval_lines = 2; // small interval
    inc_parser.parse_full(full_doc.as_bytes());

    // Invariant: initial full parses must be identical
    assert_eq!(full_parser.blocks.len(), inc_parser.blocks.len());

    // Mutate line 2
    initial_lines[2] = "Modified Line 2 content with **bold** text".to_string();
    let mutated_doc = initial_lines.join("\n") + "\n";

    full_parser.parse_full(mutated_doc.as_bytes());
    inc_parser.reparse_incremental(mutated_doc.as_bytes(), 2);

    // Invariant: incremental reparse must yield identical block count and block kinds as full reparse
    assert_eq!(full_parser.blocks.len(), inc_parser.blocks.len());
    for i in 0..full_parser.blocks.len() {
        assert_eq!(
            BlockKind::from_u8(full_parser.blocks.kind[i]),
            BlockKind::from_u8(inc_parser.blocks.kind[i]),
            "Block mismatch at index {}",
            i
        );
    }
}
