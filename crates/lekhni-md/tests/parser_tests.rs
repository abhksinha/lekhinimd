use lekhni_md::block::BlockKind;
use lekhni_md::inline::{style, InlineParser};
use lekhni_md::parser::MarkdownParser;

#[test]
fn test_parse_blocks() {
    let doc = b"# Title\n\nParagraph text here.\n\n```rust\nfn main() {}\n```\n";
    let mut parser = MarkdownParser::new();
    parser.parse_full(doc);

    assert_eq!(parser.blocks.len(), 3);
    assert_eq!(BlockKind::from_u8(parser.blocks.kind[0]), BlockKind::Heading1);
    assert_eq!(BlockKind::from_u8(parser.blocks.kind[1]), BlockKind::Paragraph);
    assert_eq!(BlockKind::from_u8(parser.blocks.kind[2]), BlockKind::FencedCode);
}

#[test]
fn test_incremental_reparse() {
    let mut parser = MarkdownParser::new();
    parser.checkpoints.interval_lines = 2; // small interval for test

    let doc = b"# Line 0\n# Line 1\n# Line 2\n# Line 3\n# Line 4\n";
    parser.parse_full(doc);
    let initial_count = parser.blocks.len();
    assert_eq!(initial_count, 5);

    // Modify line 3
    let doc_modified = b"# Line 0\n# Line 1\n# Line 2\n# Modified Line 3\n# Line 4\n";
    parser.reparse_incremental(doc_modified, 3);
    assert_eq!(parser.blocks.len(), initial_count);
}

#[test]
fn test_inline_spans() {
    let text = b"This is **bold** and `code` span.";
    let mut spans = Vec::new();
    InlineParser::parse_spans(text, &mut spans);

    assert!(spans.iter().any(|s| s.style() == style::BOLD));
    assert!(spans.iter().any(|s| s.style() == style::CODE));
}
