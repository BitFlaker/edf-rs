#[cfg(test)]
mod file_edit_tests {
    use chrono::{NaiveDate, NaiveTime};
    use std::fs::{self, exists, remove_file};
    use std::iter::repeat_n;

    use crate::EDFSpecifications;
    use crate::file::EDFFile;
    use crate::headers::annotation_list::AnnotationList;
    use crate::headers::edf_header::EDFHeader;
    use crate::headers::patient::{PatientId, Sex};
    use crate::headers::recording::RecordingId;
    use crate::headers::signal_header::SignalHeader;
    use crate::record::Record;

    #[test]
    fn test_remove_all_signals() {
        let (path_actual, path_expected) = get_paths("remove_all_signals");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();
        edf_actual.header.with_specification(EDFSpecifications::EDF);

        // Modify records
        edf_actual
            .update_record(0, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .update_record(4, generate_default_record(&edf_actual, 31))
            .unwrap();

        // Modify signals
        edf_actual.remove_signal(0).unwrap();
        edf_actual.remove_signal(0).unwrap();
        edf_actual.remove_signal(0).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Generate the expected file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);
        edf_expected
            .header
            .with_specification(EDFSpecifications::EDF);
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_upsample_update_first_record() {
        let (path_actual, path_expected) = get_paths("upsample_update_first_record");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual
            .update_record(0, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .update_record(4, generate_default_record(&edf_actual, 31))
            .unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 210;
        edf_actual.update_signal(1, patch_signal).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_upsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                3,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_upsample_update_last_record() {
        let (path_actual, path_expected) = get_paths("upsample_update_last_record");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .update_record(0, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .update_record(2, generate_default_record(&edf_actual, 31))
            .unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 210;
        edf_actual.update_signal(1, patch_signal).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_upsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_upsample_update_remove_signal() {
        let (path_actual, path_expected) = get_paths("upsample_update_remove_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .update_record(0, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(2, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(3).unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 210;
        edf_actual.update_signal(1, patch_signal).unwrap();
        edf_actual.remove_signal(0).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_upsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_upsample_update_add_signal() {
        let (path_actual, path_expected) = get_paths("upsample_update_add_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(0, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(2, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(3).unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 210;
        edf_actual.update_signal(1, patch_signal).unwrap();

        let mut new_signal = generate_default_signal2();
        new_signal
            .with_label("INSERTED".to_string())
            .with_samples_count(200);
        edf_actual.insert_signal(3, new_signal.clone()).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_upsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();
        edf_expected.insert_signal(3, new_signal).unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                    |_| repeat_n(0, 200).collect(),
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                    |_| repeat_n(0, 200).collect(),
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                    |_| repeat_n(0, 200).collect(),
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                    |_| repeat_n(0, 200).collect(),
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_downsample_remove_signal() {
        let (path_actual, path_expected) = get_paths("downsample_remove_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 61;
        edf_actual.update_signal(1, patch_signal).unwrap();
        edf_actual.remove_signal(0).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_downsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![generate_default_signal2_data_downsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![generate_default_signal2_data_downsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![generate_default_signal2_data_downsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![generate_default_signal2_data_downsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![generate_default_signal2_data_downsampled],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_upsample_remove_signal() {
        let (path_actual, path_expected) = get_paths("upsample_remove_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 210;
        edf_actual.update_signal(1, patch_signal).unwrap();
        edf_actual.remove_signal(0).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_upsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![generate_default_signal2_data_upsampled],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_upsample_add_signal() {
        let (path_actual, path_expected) = get_paths("upsample_add_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 210;
        edf_actual.update_signal(1, patch_signal).unwrap();

        let mut new_signal = generate_default_signal2();
        new_signal
            .with_label("INSERTED".to_string())
            .with_samples_count(200);
        edf_actual.insert_signal(1, new_signal.clone()).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected.insert_signal(1, new_signal).unwrap();
        edf_expected
            .insert_signal(2, generate_upsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(3, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_downsample_add_signal() {
        let (path_actual, path_expected) = get_paths("downsample_add_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        let mut patch_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        patch_signal.samples_count = 61;
        edf_actual.update_signal(1, patch_signal).unwrap();

        let mut new_signal = generate_default_signal2();
        new_signal
            .with_label("INSERTED".to_string())
            .with_samples_count(200);
        edf_actual.insert_signal(1, new_signal.clone()).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected.insert_signal(1, new_signal).unwrap();
        edf_expected
            .insert_signal(2, generate_downsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(3, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![
                    generate_default_signal1_data,
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_downsample_signal() {
        let (path_actual, path_expected) = get_paths("downsample_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        let mut new_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        new_signal.samples_count = 61;
        edf_actual.update_signal(1, new_signal).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_downsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_downsampled,
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_upsample_signal() {
        let (path_actual, path_expected) = get_paths("upsample_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        let mut new_signal = edf_actual.header.get_signals().get(1).unwrap().clone();
        new_signal.samples_count = 210;
        edf_actual.update_signal(1, new_signal).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_upsampled_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![
                    generate_default_signal1_data,
                    generate_default_signal2_data_upsampled,
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_remove_signal() {
        let (path_actual, path_expected) = get_paths("remove_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        edf_actual.remove_signal(1).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![generate_default_signal1_data],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![generate_default_signal1_data],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![generate_default_signal1_data],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![generate_default_signal1_data],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![generate_default_signal1_data],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_insert_signal() {
        let (path_actual, path_expected) = get_paths("insert_signal");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();

        // Modify signals
        let mut new_signal = generate_default_signal2();
        new_signal
            .with_label("INSERTED".to_string())
            .with_samples_count(200);
        edf_actual.insert_signal(0, new_signal.clone()).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected.insert_signal(0, new_signal).unwrap();
        edf_expected
            .insert_signal(1, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_signal2())
            .unwrap();
        edf_expected
            .insert_signal(3, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                1,
                vec![
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal1_data,
                    generate_default_signal2_data,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                30,
                vec![
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal1_data,
                    generate_default_signal2_data,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                2,
                vec![
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal1_data,
                    generate_default_signal2_data,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                31,
                vec![
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal1_data,
                    generate_default_signal2_data,
                ],
            ))
            .unwrap();
        edf_expected
            .append_record(generate_custom_signal_record(
                &edf_expected,
                4,
                vec![
                    |_| repeat_n(0, 200).collect(),
                    generate_default_signal1_data,
                    generate_default_signal2_data,
                ],
            ))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_insert_delete() {
        let (path_actual, path_expected) = get_paths("insert_delete");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .insert_record(1, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(3, generate_default_record(&edf_actual, 31))
            .unwrap();
        edf_actual.remove_record(4).unwrap();
        edf_actual
            .append_record(generate_default_record(&edf_actual, 32))
            .unwrap();
        edf_actual.remove_record(5).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_default_record(&edf_expected, 1))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 30))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 2))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 31))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 4))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_update_file() {
        let (path_actual, path_expected) = get_paths("update_file");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual
            .update_record(2, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual.remove_record(3).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_default_record(&edf_expected, 1))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 2))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 30))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_shrink_file() {
        let (path_actual, path_expected) = get_paths("shrink_file");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual.remove_record(0).unwrap();
        edf_actual.remove_record(3).unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_default_record(&edf_expected, 1))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 2))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 3))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_extend_file() {
        let (path_actual, path_expected) = get_paths("extend_file");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual
            .insert_record(2, generate_default_record(&edf_actual, 30))
            .unwrap();
        edf_actual
            .insert_record(5, generate_default_record(&edf_actual, 31))
            .unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_default_record(&edf_expected, 0))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 1))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 30))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 2))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 3))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 31))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 4))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    #[test]
    fn test_update_record() {
        let (path_actual, path_expected) = get_paths("update_record");

        // ============== ACT ===============

        let mut edf_actual = EDFFile::open(&path_actual).unwrap();

        // Modify records
        edf_actual
            .update_record(2, generate_default_record(&edf_actual, 30))
            .unwrap();

        // Apply modifications
        edf_actual.save().unwrap();

        // ============== EXPECTED ===============

        // Create new EDF file
        let mut edf_expected = EDFFile::new(&path_expected).unwrap();
        configure_default_header(&mut edf_expected.header);

        // Create signals
        edf_expected
            .insert_signal(0, generate_default_signal1())
            .unwrap();
        edf_expected
            .insert_signal(1, generate_default_signal2())
            .unwrap();
        edf_expected
            .insert_signal(2, generate_default_annotations())
            .unwrap();

        // Create records
        edf_expected
            .append_record(generate_default_record(&edf_expected, 0))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 1))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 30))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 3))
            .unwrap();
        edf_expected
            .append_record(generate_default_record(&edf_expected, 4))
            .unwrap();

        // Create the file
        edf_expected.save().unwrap();

        // ============== ASSERT ===============

        let data_expected = fs::read(&path_expected).unwrap();
        let data_actual = fs::read(&path_actual).unwrap();
        assert_eq!(data_expected, data_actual);

        // ============== CLEANUP ==============

        remove_file(path_expected).unwrap();
        remove_file(path_actual).unwrap();
    }

    // =====================================
    // =              HELPERS              =
    // =====================================

    fn get_paths(name: &str) -> (String, String) {
        let name_actual = name.to_string() + "_actual";
        let name_expected = name.to_string() + "_expected";
        let path_actual = generate_test_edf(&name_actual);

        // Clean previous test file
        let path_expected = generate_file_path(&name_expected);
        if exists(&path_expected).unwrap() {
            remove_file(&path_expected).unwrap();
        }

        (path_actual, path_expected)
    }

    fn generate_default_signal1() -> SignalHeader {
        let mut signal1 = SignalHeader::new();
        signal1
            .with_label("Signal1".to_string())
            .with_transducer("Unknown1".to_string())
            .with_physical_dimension("uV".to_string())
            .with_prefilter("".to_string())
            .with_physical_range(-1234.0, 1233.0)
            .with_digital_range(-1024, 1024)
            .with_samples_count(100);

        signal1
    }

    fn generate_default_signal2() -> SignalHeader {
        let mut signal2 = SignalHeader::new();
        signal2
            .with_label("Signal2".to_string())
            .with_transducer("Unknown2".to_string())
            .with_physical_dimension("uV".to_string())
            .with_prefilter("HP:0.1Hz LP:75Hz".to_string())
            .with_physical_range(-512.0, 512.0)
            .with_digital_range(-64, 64)
            .with_samples_count(127);

        signal2
    }

    fn generate_upsampled_signal2() -> SignalHeader {
        let mut signal2 = generate_default_signal2();
        signal2.with_samples_count(210);
        signal2
    }

    fn generate_downsampled_signal2() -> SignalHeader {
        let mut signal2 = generate_default_signal2();
        signal2.with_samples_count(61);
        signal2
    }

    fn generate_default_annotations() -> SignalHeader {
        SignalHeader::new_annotation(80)
    }

    fn generate_custom_signal_record<T: Fn(usize) -> Vec<i16>>(
        edf: &EDFFile,
        index: usize,
        samples: Vec<T>,
    ) -> Record {
        let mut record = edf.header.create_record();
        record.signal_samples = samples.iter().map(|genr| genr(index)).collect();
        record.annotations = vec![vec![
            AnnotationList::new(0.0, 0.0, vec![format!("GlobalAnnotation {}", index)]).unwrap(),
        ]];

        record
    }

    fn generate_default_record(edf: &EDFFile, index: usize) -> Record {
        let mut record = edf.header.create_record();
        record.signal_samples = vec![
            generate_default_signal1_data(index),
            generate_default_signal2_data(index),
        ];
        record.annotations = vec![vec![
            AnnotationList::new(0.0, 0.0, vec![format!("GlobalAnnotation {}", index)]).unwrap(),
        ]];

        record
    }

    fn generate_default_signal2_data_upsampled(index: usize) -> Vec<i16> {
        let mut default_data = generate_default_signal2_data(index);
        default_data.extend(repeat_n(0, 210 - default_data.len()));
        default_data
    }

    fn generate_default_signal2_data_downsampled(index: usize) -> Vec<i16> {
        ((0 + 25 * index as i16)..(61 + 25 * index as i16)).collect()
    }

    fn generate_default_signal2_data(index: usize) -> Vec<i16> {
        ((0 + 25 * index as i16)..(127 + 25 * index as i16)).collect()
    }

    fn generate_default_signal1_data(index: usize) -> Vec<i16> {
        ((0 + 25 * index as i16)..(100 + 25 * index as i16)).collect()
    }

    fn configure_default_header(edf_header: &mut EDFHeader) {
        edf_header
            .with_specification(EDFSpecifications::EDFPlus)
            .with_is_continuous(true)
            .with_patient_id(PatientId {
                code: Some("PAT-CODE1".to_string()),
                name: Some("Pat-NAME".to_string()),
                date: Some(NaiveDate::from_ymd_opt(2001, 07, 11).unwrap()),
                sex: Some(Sex::Male),
                additional: Vec::new(),
            })
            .with_recording_id(RecordingId {
                admin_code: Some("REC-CODE1".to_string()),
                equipment: Some("EQUIPMENT".to_string()),
                technician: Some("TECHNICIAN".to_string()),
                startdate: Some(NaiveDate::from_ymd_opt(2026, 02, 13).unwrap()),
                additional: Vec::new(),
            })
            .with_start_date(NaiveDate::from_ymd_opt(2026, 02, 13).unwrap())
            .with_start_time(NaiveTime::from_hms_opt(17, 30, 0).unwrap())
            .with_record_duration(1.0);
    }

    fn generate_file_path(name: &str) -> String {
        format!("code_tests/test_{}.edf", name)
    }

    // =====================================
    // =         DEFAULT TEST FILE         =
    // =====================================

    fn generate_test_edf(name: &str) -> String {
        // Clean previous test file
        let path = generate_file_path(name);
        if exists(&path).unwrap() {
            remove_file(&path).unwrap();
        }

        // Create new EDF file
        let mut edf = EDFFile::new(&path).unwrap();

        // Configure header with defaults
        configure_default_header(&mut edf.header);

        // Insert the 2 default signals and the annotations
        edf.insert_signal(0, generate_default_signal1()).unwrap();
        edf.insert_signal(1, generate_default_signal2()).unwrap();
        edf.insert_signal(2, generate_default_annotations())
            .unwrap();

        // Insert 5 default records
        edf.append_record(generate_default_record(&edf, 0)).unwrap();
        edf.append_record(generate_default_record(&edf, 1)).unwrap();
        edf.append_record(generate_default_record(&edf, 2)).unwrap();
        edf.append_record(generate_default_record(&edf, 3)).unwrap();
        edf.append_record(generate_default_record(&edf, 4)).unwrap();

        // Save the file
        edf.save().unwrap();

        // Return the path to the file
        path
    }
}

#[cfg(test)]
mod file_read_tests {
    use crate::file::EDFFile;
    use crate::headers::annotation_list::AnnotationList;
    use crate::record::RelativeRecordData;
    use std::error::Error;

    #[test]
    fn test_normal_edfp_read() -> Result<(), Box<dyn Error>> {
        let mut edf = EDFFile::open("code_tests/EDF+ normal.edf")?;

        // Read full 1st record
        let rec2 = edf.read_seconds_approx(1.0)?;
        assert_eq!(
            rec2.signal_samples,
            vec![
                vec![RelativeRecordData {
                    offset: 0.0,
                    signal_samples: vec![
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
                        40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58,
                        59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77,
                        78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96,
                        97, 98, 99
                    ]
                }],
                vec![RelativeRecordData {
                    offset: 0.0,
                    signal_samples: vec![
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
                        40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58,
                        59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77,
                        78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96,
                        97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
                        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126
                    ]
                }],
            ]
        );

        assert_eq!(
            rec2.annotations,
            vec![vec![AnnotationList::new(
                0.0,
                0.0,
                vec!["SomeText".to_string()]
            )?]]
        );

        // Read first quarter of 2nd record
        let rec2 = edf.read_seconds_approx(0.25)?;
        assert_eq!(
            rec2.signal_samples,
            vec![
                vec![RelativeRecordData {
                    offset: 1.0,
                    signal_samples: vec![
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24
                    ]
                }],
                vec![RelativeRecordData {
                    offset: 1.0,
                    signal_samples: vec![
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29, 30
                    ]
                }],
            ]
        );
        assert_eq!(
            rec2.annotations,
            vec![vec![AnnotationList::new(
                0.0,
                0.0,
                vec!["SomeText".to_string()]
            )?]]
        );

        // Read second quarter of 2nd record
        let rec2 = edf.read_seconds_approx(0.25)?;
        assert_eq!(
            rec2.signal_samples,
            vec![
                vec![RelativeRecordData {
                    offset: 1.25,
                    signal_samples: vec![
                        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43,
                        44, 45, 46, 47, 48, 49
                    ]
                }],
                vec![RelativeRecordData {
                    offset: 1.25,
                    signal_samples: vec![
                        31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
                        50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62
                    ]
                }],
            ]
        );
        assert_eq!(
            rec2.annotations,
            vec![vec![AnnotationList::new(
                0.0,
                0.0,
                vec!["SomeText".to_string()]
            )?]]
        );

        // Read third quarter of 2nd record
        let rec2 = edf.read_seconds_approx(0.25)?;
        assert_eq!(
            rec2.signal_samples,
            vec![
                vec![RelativeRecordData {
                    offset: 1.5,
                    signal_samples: vec![
                        50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                        69, 70, 71, 72, 73, 74
                    ]
                }],
                vec![RelativeRecordData {
                    offset: 1.5,
                    signal_samples: vec![
                        63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81,
                        82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94
                    ]
                }],
            ]
        );
        assert_eq!(
            rec2.annotations,
            vec![vec![AnnotationList::new(
                0.0,
                0.0,
                vec!["SomeText".to_string()]
            )?]]
        );

        // Read fourth quarter of 2nd record
        let rec2 = edf.read_seconds_approx(0.25)?;
        assert_eq!(
            rec2.signal_samples,
            vec![
                vec![RelativeRecordData {
                    offset: 1.75,
                    signal_samples: vec![
                        75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93,
                        94, 95, 96, 97, 98, 99
                    ]
                }],
                vec![RelativeRecordData {
                    offset: 1.75,
                    signal_samples: vec![
                        95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110,
                        111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125,
                        126
                    ]
                }],
            ]
        );
        assert_eq!(
            rec2.annotations,
            vec![vec![AnnotationList::new(
                0.0,
                0.0,
                vec!["SomeText".to_string()]
            )?]]
        );

        let rec2 = edf.read_seconds_approx(1.0)?;
        assert_eq!(
            rec2.signal_samples,
            vec![
                vec![RelativeRecordData {
                    offset: 2.0,
                    signal_samples: vec![
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
                        40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58,
                        59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77,
                        78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96,
                        97, 98, 99
                    ]
                }],
                vec![RelativeRecordData {
                    offset: 2.0,
                    signal_samples: vec![
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
                        40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58,
                        59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77,
                        78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96,
                        97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
                        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126
                    ]
                }],
            ]
        );
        assert_eq!(
            rec2.annotations,
            vec![vec![AnnotationList::new(
                0.0,
                0.0,
                vec!["SomeText".to_string()]
            )?]]
        );

        Ok(())
    }
}
