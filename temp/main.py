#!/usr/bin/env python3

from sigmf import SigMFFile, sigmffile

sigmf_files = ['0x0_0.sigmf', '0x1_0.sigmf', '0x2_0.sigmf', '0x3_0.sigmf']

for file in sigmf_files:
    print(f"File: {file}")
    signal = sigmffile.fromfile(file)
    
    print(signal)
    
    # Get some metadata and all annotations
    sample_rate = signal.get_global_field(SigMFFile.SAMPLE_RATE_KEY)
    sample_count = signal.sample_count
    signal_duration = sample_count / sample_rate
    annotations = signal.get_annotations()
    
    print(f"Sample Rate: {sample_rate}")
    print(f"Sample Count: {sample_count}")
    print(f"Signal Duration: {signal_duration}")
    print(f"Annotations: {len(annotations)}\n\n")

    # Iterate over annotations
    for adx, annotation in enumerate(annotations):
        annotation_start_idx = annotation[SigMFFile.START_INDEX_KEY]
        annotation_length = annotation[SigMFFile.LENGTH_INDEX_KEY]
        annotation_comment = annotation.get(SigMFFile.COMMENT_KEY, "[annotation {}]".format(adx))

        # Get capture info associated with the start of annotation
        capture = signal.get_capture_info(annotation_start_idx)
        freq_center = capture.get(SigMFFile.FREQUENCY_KEY, 0)
        freq_min = freq_center - 0.5*sample_rate
        freq_max = freq_center + 0.5*sample_rate

        # Get frequency edges of annotation (default to edges of capture)
        freq_start = annotation.get(SigMFFile.FLO_KEY)
        freq_stop = annotation.get(SigMFFile.FHI_KEY)

        # Get the samples corresponding to annotation
        samples = signal.read_samples(annotation_start_idx, annotation_length)
        
        print(f"Annotation: {annotation_comment}")
        print(f"  Start: {annotation_start_idx}")
        print(f"  Length: {annotation_length}")
        print(f"  Center Frequency: {freq_center}")
        print(f"  Frequency Range: {freq_min} - {freq_max}")
        print(f"  Frequency Edges: {freq_start} - {freq_stop}")
        print(f"  Samples: {samples}")
        print(f"  Capture Info: {capture}")
        print("\n")

