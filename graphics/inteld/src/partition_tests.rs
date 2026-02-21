//! Unit tests for GPU partition management.

#[cfg(test)]
mod tests {
    use crate::partition::{GpuPartition, PartitionTable};

    fn make_partition(id: usize, core_mask: u64, vram_start: u64, vram_size: u64) -> GpuPartition {
        GpuPartition {
            id,
            core_mask,
            vram_start,
            vram_size,
        }
    }

    // ── GpuPartition helpers ──────────────────────────────────────────────────

    #[test]
    fn vram_end_is_start_plus_size() {
        let p = make_partition(0, 0xFF, 0x1000, 0x2000);
        assert_eq!(p.vram_end(), 0x3000);
    }

    #[test]
    fn vram_overlaps_detects_overlap() {
        let a = make_partition(0, 0x0F, 0x0000, 0x2000);
        let b = make_partition(1, 0xF0, 0x1000, 0x2000); // overlaps a
        assert!(a.vram_overlaps(&b));
        assert!(b.vram_overlaps(&a));
    }

    #[test]
    fn vram_overlaps_adjacent_is_not_overlap() {
        let a = make_partition(0, 0x0F, 0x0000, 0x1000);
        let b = make_partition(1, 0xF0, 0x1000, 0x1000); // adjacent, not overlapping
        assert!(!a.vram_overlaps(&b));
        assert!(!b.vram_overlaps(&a));
    }

    #[test]
    fn cores_overlap_detects_shared_bits() {
        let a = make_partition(0, 0b0011, 0x0000, 0x1000);
        let b = make_partition(1, 0b0110, 0x1000, 0x1000); // bit 1 shared
        assert!(a.cores_overlap(&b));
    }

    #[test]
    fn cores_overlap_disjoint_masks() {
        let a = make_partition(0, 0b0011, 0x0000, 0x1000);
        let b = make_partition(1, 0b1100, 0x1000, 0x1000);
        assert!(!a.cores_overlap(&b));
    }

    // ── PartitionTable ────────────────────────────────────────────────────────

    const TOTAL_VRAM: u64 = 0x1000_0000; // 256 MiB

    #[test]
    fn add_partition_accepts_non_overlapping() {
        let table = PartitionTable::new(TOTAL_VRAM);
        assert!(table
            .add_partition(make_partition(0, 0x0F, 0x0000_0000, 0x0800_0000))
            .is_ok());
        assert!(table
            .add_partition(make_partition(1, 0xF0, 0x0800_0000, 0x0800_0000))
            .is_ok());
    }

    #[test]
    fn add_partition_rejects_vram_overlap() {
        let table = PartitionTable::new(TOTAL_VRAM);
        assert!(table
            .add_partition(make_partition(0, 0x0F, 0x0000_0000, 0x0800_0000))
            .is_ok());
        // Overlaps in VRAM.
        assert!(table
            .add_partition(make_partition(1, 0xF0, 0x0400_0000, 0x0800_0000))
            .is_err());
    }

    #[test]
    fn add_partition_rejects_core_overlap() {
        let table = PartitionTable::new(TOTAL_VRAM);
        assert!(table
            .add_partition(make_partition(0, 0xFF, 0x0000_0000, 0x0800_0000))
            .is_ok());
        // Overlaps in core mask.
        assert!(table
            .add_partition(make_partition(1, 0x0F, 0x0800_0000, 0x0800_0000))
            .is_err());
    }

    #[test]
    fn add_partition_rejects_out_of_range_vram() {
        let table = PartitionTable::new(TOTAL_VRAM);
        // vram_start + vram_size > total_vram.
        assert!(table
            .add_partition(make_partition(0, 0xFF, 0x0F00_0000, 0x0200_0000))
            .is_err());
    }

    #[test]
    fn add_partition_rejects_zero_size() {
        let table = PartitionTable::new(TOTAL_VRAM);
        assert!(table.add_partition(make_partition(0, 0xFF, 0, 0)).is_err());
    }

    #[test]
    fn add_partition_rejects_zero_core_mask() {
        let table = PartitionTable::new(TOTAL_VRAM);
        assert!(table
            .add_partition(make_partition(0, 0, 0, 0x1000))
            .is_err());
    }

    #[test]
    fn remove_partition_removes_by_id() {
        let table = PartitionTable::new(TOTAL_VRAM);
        table
            .add_partition(make_partition(0, 0x0F, 0x0000_0000, 0x0800_0000))
            .unwrap();
        table
            .add_partition(make_partition(1, 0xF0, 0x0800_0000, 0x0800_0000))
            .unwrap();
        table.remove_partition(0);
        assert!(table.get_partition(0).is_none());
        assert!(table.get_partition(1).is_some());
    }

    #[test]
    fn get_partition_returns_correct_data() {
        let table = PartitionTable::new(TOTAL_VRAM);
        let p = make_partition(42, 0xAB, 0x0000_0000, 0x0100_0000);
        table.add_partition(p.clone()).unwrap();
        let got = table.get_partition(42).unwrap();
        assert_eq!(got.id, 42);
        assert_eq!(got.core_mask, 0xAB);
        assert_eq!(got.vram_start, 0x0000_0000);
        assert_eq!(got.vram_size, 0x0100_0000);
    }

    #[test]
    fn after_remove_slot_can_be_reused() {
        let table = PartitionTable::new(TOTAL_VRAM);
        table
            .add_partition(make_partition(0, 0x0F, 0x0000_0000, 0x0800_0000))
            .unwrap();
        table.remove_partition(0);
        // Same VRAM and core range should now be accepted.
        assert!(table
            .add_partition(make_partition(1, 0x0F, 0x0000_0000, 0x0800_0000))
            .is_ok());
    }
}
