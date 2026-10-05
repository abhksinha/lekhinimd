use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_ui::preview::{ClickableTarget, PreviewRenderer};

#[test]
fn test_block_layout_multi_line_height_measurement() {
    let doc = b"# Header Title\n\nParagraph line 1\nParagraph line 2\nParagraph line 3\n\n```rust\nlet x = 1;\nlet y = 2;\nlet z = 3;\n```\n\n| H1 | H2 |\n|----|----|\n| D1 | D2 |\n";
    let mut parser = MarkdownParser::new();
    parser.parse_full(doc);

    let layouts = PreviewRenderer::compute_layouts(&parser.blocks, doc, 800);
    assert!(layouts.len() >= 4);

    // Heading1: 44px
    assert_eq!(layouts[0].height, 44);

    // Multi-line paragraph: 3 lines * 18 + 8 = 62px
    assert!(layouts[1].height >= 50, "Paragraph should measure all lines, got {}", layouts[1].height);

    // Fenced code: 3 code lines * 18 + 24 + 8 = 86px
    assert!(layouts[2].height >= 70, "Fenced code should measure all code lines, got {}", layouts[2].height);

    // Table: 2 rows (header + data) * 24 + 10 + 6 = 64px
    assert!(layouts[3].height >= 50, "Table should measure all rows, got {}", layouts[3].height);
}

#[test]
fn test_clickable_targets_detection() {
    let doc = b"# Main Heading\n\n- [ ] Unchecked task\n- [x] Completed task\n\nVisit [Lekhni Docs](https://github.com/abhksinha/lekhinimd) for details.\n";
    let mut parser = MarkdownParser::new();
    parser.parse_full(doc);

    let mut fb = vec![0u32; 800 * 600];
    let mut canvas = Canvas::new(&mut fb, 800, 600);

    let targets = PreviewRenderer::render_preview_with_content(
        &mut canvas,
        &parser.blocks,
        doc,
        0,
        800,
        600,
    );

    // Should detect Heading, 2 Checkboxes (one unchecked, one checked), and 1 Link
    let mut has_heading = false;
    let mut has_unchecked = false;
    let mut has_checked = false;
    let mut has_link = false;

    for target in targets {
        match target {
            ClickableTarget::Heading { .. } => has_heading = true,
            ClickableTarget::Checkbox { is_checked: false, .. } => has_unchecked = true,
            ClickableTarget::Checkbox { is_checked: true, .. } => has_checked = true,
            ClickableTarget::Link { url, .. } => {
                if url == "https://github.com/abhksinha/lekhinimd" {
                    has_link = true;
                }
            }
        }
    }

    assert!(has_heading, "Heading clickable target must be generated");
    assert!(has_unchecked, "Unchecked task target must be generated");
    assert!(has_checked, "Checked task target must be generated");
    assert!(has_link, "Link target must be generated with correct URL");
}
