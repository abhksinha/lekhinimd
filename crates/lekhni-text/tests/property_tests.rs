use lekhni_text::buffer::PieceTable;

#[test]
fn test_piece_table_property_random_edits() {
    let mut pt = PieceTable::new(b"Initial test buffer content.\n");
    let mut reference = b"Initial test buffer content.\n".to_vec();

    // Pseudo-random linear congruential generator (LCG) for deterministic no-dependency testing
    let mut state: u64 = 42;
    let mut next_rand = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 32) as usize
    };

    for _ in 0..100 {
        let op = next_rand() % 2;
        if op == 0 || reference.is_empty() {
            // Insertion
            let insert_pos = if reference.is_empty() { 0 } else { next_rand() % (reference.len() + 1) };
            let sample_data: &[u8] = match next_rand() % 4 {
                0 => b"alpha ",
                1 => b"beta\n",
                2 => b"gamma ",
                _ => b"delta\n",
            };
            pt.insert(insert_pos, sample_data);
            reference.splice(insert_pos..insert_pos, sample_data.iter().copied());
        } else {
            // Deletion
            let del_pos = next_rand() % reference.len();
            let max_len = reference.len() - del_pos;
            let del_len = 1 + (next_rand() % max_len.min(8));
            pt.delete(del_pos, del_len);
            reference.drain(del_pos..del_pos + del_len);
        }

        // Invariant check: lengths and contents must match byte-for-byte
        assert_eq!(pt.len(), reference.len());
        let mut actual = vec![0u8; pt.len()];
        let n = pt.copy_range(0, pt.len(), &mut actual);
        assert_eq!(&actual[..n], &reference[..]);
    }
}
