import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/registry

/// Converts a `CodeString` value to a descriptive string if one is available.
///
/// This is not an exhaustive list, but rather aims to add value when printing
/// data sets by describing commonly seen and non-obvious `CodeString`s.
///
pub fn describe(value: String, tag: DataElementTag) -> Result(String, Nil) {
  case tag {
    tag if tag == registry.modality.tag ->
      case value {
        "ANN" -> Ok("Annotation")
        "AR" -> Ok("Autorefraction")
        "ASMT" -> Ok("Content Assessment Results")
        "AU" -> Ok("Audio")
        "BDUS" -> Ok("Bone Densitometry (ultrasound)")
        "BI" -> Ok("Biomagnetic imaging")
        "BMD" -> Ok("Bone Densitometry (X-Ray)")
        "CFM" -> Ok("Confocal Microscopy")
        "CR" -> Ok("Computed Radiography")
        "CT" -> Ok("Computed Tomography")
        "CTPROTOCOL" -> Ok("CT Protocol (Performed)")
        "DMS" -> Ok("Dermoscopy")
        "DG" -> Ok("Diaphanography")
        "DOC" -> Ok("Document")
        "DX" -> Ok("Digital Radiography")
        "ECG" -> Ok("Electrocardiography")
        "EEG" -> Ok("Electroencephalography")
        "EMG" -> Ok("Electromyography")
        "EOG" -> Ok("Electrooculography")
        "EPS" -> Ok("Cardiac Electrophysiology")
        "ES" -> Ok("Endoscopy")
        "FID" -> Ok("Fiducials")
        "GM" -> Ok("General Microscopy")
        "HC" -> Ok("Hard Copy")
        "HD" -> Ok("Hemodynamic Waveform")
        "IO" -> Ok("Intra-Oral Radiography")
        "IOL" -> Ok("Intraocular Lens Data")
        "IVOCT" -> Ok("Intravascular Optical Coherence Tomography")
        "IVUS" -> Ok("Intravascular Ultrasound")
        "KER" -> Ok("Keratometry")
        "KO" -> Ok("Key Object Selection")
        "LEN" -> Ok("Lensometry")
        "LS" -> Ok("Laser surface scan")
        "MG" -> Ok("Mammography")
        "MR" -> Ok("Magnetic Resonance")
        "M3D" -> Ok("Model for 3D Manufacturing")
        "NM" -> Ok("Nuclear Medicine")
        "OAM" -> Ok("Ophthalmic Axial Measurements")
        "OCT" -> Ok("Optical Coherence Tomography (non-Ophthalmic)")
        "OP" -> Ok("Ophthalmic Photography")
        "OPM" -> Ok("Ophthalmic Mapping")
        "OPT" -> Ok("Ophthalmic Tomography")
        "OPTBSV" -> Ok("Ophthalmic Tomography B-scan Volume Analysis")
        "OPTENF" -> Ok("Ophthalmic Tomography En Face")
        "OPV" -> Ok("Ophthalmic Visual Field")
        "OSS" -> Ok("Optical Surface Scan")
        "OT" -> Ok("Other")
        "PA" -> Ok("Photoacoustic")
        "PLAN" -> Ok("Plan")
        "POS" -> Ok("Position Sensor")
        "PR" -> Ok("Presentation State")
        "PT" -> Ok("Positron emission tomography (PET)")
        "PX" -> Ok("Panoramic X-Ray")
        "REG" -> Ok("Registration")
        "RESP" -> Ok("Respiratory Waveform")
        "RF" -> Ok("Radio Fluoroscopy")
        "RG" -> Ok("Radiographic imaging (conventional film/screen)")
        "RTDOSE" -> Ok("Radiotherapy Dose")
        "RTIMAGE" -> Ok("Radiotherapy Image")
        "RTINTENT" -> Ok("Radiotherapy Intent")
        "RTPLAN" -> Ok("Radiotherapy Plan")
        "RTRAD" -> Ok("RT Radiation")
        "RTRECORD" -> Ok("RT Treatment Record")
        "RTSEGANN" -> Ok("Radiotherapy Segment Annotation")
        "RTSTRUCT" -> Ok("Radiotherapy Structure Set")
        "RWV" -> Ok("Real World Value Map")
        "SEG" -> Ok("Segmentation")
        "SM" -> Ok("Slide Microscopy")
        "SMR" -> Ok("Stereometric Relationship")
        "SR" -> Ok("SR Document")
        "SRF" -> Ok("Subjective Refraction")
        "STAIN" -> Ok("Automated Slide Stainer")
        "TEXTUREMAP" -> Ok("Texture Map")
        "TG" -> Ok("Thermography")
        "US" -> Ok("Ultrasound")
        "VA" -> Ok("Visual Acuity")
        "XA" -> Ok("X-Ray Angiography")
        "XAPROTOCOL" -> Ok("XA Protocol (Performed)")
        "XC" -> Ok("External-camera Photography")
        _ -> Error(Nil)
      }

    tag if tag == registry.patient_sex.tag ->
      case value {
        "M" -> Ok("Male")
        "F" -> Ok("Female")
        "O" -> Ok("Other")
        _ -> Error(Nil)
      }

    tag if tag == registry.conversion_type.tag ->
      case value {
        "DV" -> Ok("Digitized Video")
        "DI" -> Ok("Digital Interface")
        "DF" -> Ok("Digitized Film")
        "WSD" -> Ok("Workstation")
        "SD" -> Ok("Scanned Document")
        "SI" -> Ok("Scanned Image")
        "DRW" -> Ok("Drawing")
        "SYN" -> Ok("Synthetic Image")
        _ -> Error(Nil)
      }

    tag if tag == registry.scanning_sequence.tag ->
      case value {
        "SE" -> Ok("Spin Echo")
        "IR" -> Ok("Inversion Recovery")
        "GR" -> Ok("Gradient Recalled")
        "EP" -> Ok("Echo Planar")
        "RM" -> Ok("Research Mode")
        _ -> Error(Nil)
      }

    tag if tag == registry.sequence_variant.tag ->
      case value {
        "SK" -> Ok("Segmented k-space")
        "MTC" -> Ok("Magnetization transfer contrast")
        "SS" -> Ok("Steady state")
        "TRSS" -> Ok("Time reversed steady state")
        "SP" -> Ok("Spoiled")
        "MP" -> Ok("MAG prepared")
        "OSP" -> Ok("Oversampling phase")
        "NONE" -> Ok("No sequence variant")
        _ -> Error(Nil)
      }

    tag if tag == registry.scan_options.tag ->
      case value {
        "PER" -> Ok("Phase Encode Reordering")
        "RG" -> Ok("Respiratory Gating")
        "CG" -> Ok("Cardiac Gating")
        "PPG" -> Ok("Peripheral Pulse Gating")
        "FC" -> Ok("Flow Compensation")
        "PFF" -> Ok("Partial Fourier - Frequency")
        "PFP" -> Ok("Partial Fourier - Phase")
        "SP" -> Ok("Spatial Presaturation")
        "FS" -> Ok("Fat Saturation")
        _ -> Error(Nil)
      }

    tag if tag == registry.acquisition_termination_condition.tag ->
      case value {
        "CNTS" -> Ok("Preset counts was reached")
        "DENS" -> Ok("Preset count density (counts/sec) was reached")
        "RDD" ->
          "Preset relative count density difference (change in counts/sec) was reached"
          |> Ok
        "MANU" -> Ok("Acquisition was terminated manually")
        "OVFL" -> Ok("Data overflow occurred")
        "TIME" -> Ok("Preset time limit was reached")
        "CARD_TRIG" -> Ok("Preset number of cardiac triggers was reached")
        "RESP_TRIG" -> Ok("Preset number of respiratory triggers was reached")
        _ -> Error(Nil)
      }

    tag if tag == registry.rotation_direction.tag ->
      case value {
        "CW" -> Ok("Clockwise")
        "CC" -> Ok("Counter clockwise")
        _ -> Error(Nil)
      }

    tag if tag == registry.radiation_setting.tag ->
      case value {
        "SC" ->
          "Low dose exposure generally corresponding to fluoroscopic settings"
          |> Ok
        "GR" -> Ok("High dose for diagnostic quality image acquisition")
        _ -> Error(Nil)
      }

    tag if tag == registry.collimator_type.tag ->
      case value {
        "PARA" -> Ok("Parallel (default)")
        "PINH" -> Ok("Pinhole")
        "FANB" -> Ok("Fan-beam")
        "CONE" -> Ok("Cone-beam")
        "SLNT" -> Ok("Slant hole")
        "ASTG" -> Ok("Astigmatic")
        "DIVG" -> Ok("Diverging")
        "NONE" -> Ok("No collimator")
        "UNKN" -> Ok("Unknown")
        _ -> Error(Nil)
      }

    tag if tag == registry.whole_body_technique.tag ->
      case value {
        "1PS" -> Ok("One pass")
        "2PS" -> Ok("Two pass")
        "PCN" -> Ok("Patient contour following employed")
        "MSP" -> Ok("Multiple static frames collected into a whole body frame")
        _ -> Error(Nil)
      }

    tag if tag == registry.patient_position.tag ->
      case value {
        "HFP" -> Ok("Head First-Prone")
        "HFS" -> Ok("Head First-Supine")
        "HFDR" -> Ok("Head First-Decubitus Right")
        "HFDL" -> Ok("Head First-Decubitus Left")
        "FFDR" -> Ok("Feet First-Decubitus Right")
        "FFDL" -> Ok("Feet First-Decubitus Left")
        "FFP" -> Ok("Feet First-Prone")
        "FFS" -> Ok("Feet First-Supine")
        "LFP" -> Ok("Left First-Prone")
        "LFS" -> Ok("Left First-Supine")
        "RFP" -> Ok("Right First-Prone")
        "RFS" -> Ok("Right First-Supine")
        "AFDR" -> Ok("Anterior First-Decubitus Right")
        "AFDL" -> Ok("Anterior First-Decubitus Left")
        "PFDR" -> Ok("Posterior First-Decubitus Right")
        "PFDL" -> Ok("Posterior First-Decubitus Left")
        _ -> Error(Nil)
      }

    tag if tag == registry.view_position.tag ->
      case value {
        "AP" -> Ok("Anterior/Posterior")
        "PA" -> Ok("Posterior/Anterior")
        "LL" -> Ok("Left Lateral")
        "RL" -> Ok("Right Lateral")
        "RLD" -> Ok("Right Lateral Decubitus")
        "LLD" -> Ok("Left Lateral Decubitus")
        "RLO" -> Ok("Right Lateral Oblique")
        "LLO" -> Ok("Left Lateral Oblique")
        _ -> Error(Nil)
      }

    tag if tag == registry.image_laterality.tag ->
      case value {
        "R" -> Ok("Right")
        "L" -> Ok("Left")
        "U" -> Ok("Unpaired")
        "B" -> Ok("Both left and right")
        _ -> Error(Nil)
      }

    tag if tag == registry.multienergy_detector_type.tag ->
      case value {
        "INTEGRATING" ->
          Ok("Physical detector integrates the full X-Ray spectrum")
        "MULTILAYER" ->
          "Physical detector layers absorb different parts of the X-Ray spectrum"
          |> Ok
        "PHOTON_COUNTING" ->
          "Physical detector counts photons with energy discrimination capability"
          |> Ok
        _ -> Error(Nil)
      }

    tag if tag == registry.corrected_image.tag ->
      case value {
        "UNIF" -> Ok("Flood corrected")
        "COR" -> Ok("Center of rotation corrected")
        "NCO" -> Ok("Non-circular orbit corrected")
        "DECY" -> Ok("Decay corrected")
        "ATTN" -> Ok("Attenuation corrected")
        "SCAT" -> Ok("Scatter corrected")
        "DTIM" -> Ok("Dead time corrected")
        "NRGY" -> Ok("Energy corrected")
        "LIN" -> Ok("Linearity corrected")
        "MOTN" -> Ok("Motion corrected")
        "CLN" -> Ok("Count loss normalization")
        _ -> Error(Nil)
      }

    tag if tag == registry.pixel_intensity_relationship.tag ->
      case value {
        "LIN" -> Ok("Approximately proportional to X-Ray beam intensity")
        "LOG" -> Ok("Non-linear \"Log Function\"")
        "OTHER" -> Ok("Not proportional to X-Ray beam intensity")
        _ -> Error(Nil)
      }

    tag if tag == registry.lossy_image_compression.tag ->
      case value {
        "00" -> Ok("Image has not been subjected to lossy compression")
        "01" -> Ok("Image has been subjected to lossy compression")
        _ -> Error(Nil)
      }

    tag if tag == registry.lossy_image_compression_method.tag ->
      case value {
        "ISO_10918_1" -> Ok("JPEG Lossy Compression [ISO/IEC 10918-1]")
        "ISO_14495_1" ->
          Ok("JPEG-LS Near-lossless Compression [ISO/IEC 14495-1]")
        "ISO_15444_1" ->
          Ok("JPEG 2000 Irreversible Compression [ISO/IEC 15444-1]")
        "ISO_15444_15" ->
          "High-Throughput JPEG 2000 Irreversible Compression [ISO/IEC 15444-15]"
          |> Ok
        "ISO_13818_2" -> Ok("MPEG2 Compression [ISO/IEC 13818-2]")
        "ISO_14496_10" -> Ok("MPEG-4 AVC/H.264 Compression [ISO/IEC 14496-10]")
        "ISO_23008_2" -> Ok("HEVC/H.265 Lossy Compression [ISO/IEC 23008-2]")
        _ -> Error(Nil)
      }

    tag if tag == registry.universal_entity_id_type.tag ->
      case value {
        "DNS" -> Ok("An Internet dotted name. Either in ASCII or as integers")
        "EUI64" -> Ok("An IEEE Extended Unique Identifier")
        "ISO" -> Ok("An International Standards Organization Object Identifier")
        "URI" -> Ok("Uniform Resource Identifier")
        "UUID" -> Ok("The DCE Universal Unique Identifier")
        "X400" -> Ok("An X.400 MHS identifier")
        "X500" -> Ok("An X.500 directory name")
        _ -> Error(Nil)
      }

    tag if tag == registry.slice_progression_direction.tag ->
      case value {
        "APEX_TO_BASE" -> Ok("Apex to base")
        "BASE_TO_APEX" -> Ok("Base to apex")
        "ANT_TO_INF" -> Ok("Anterior to inferior")
        "INF_TO_ANT" -> Ok("Inferior to anterior")
        "SEPTUM_TO_WALL" -> Ok("Septum to lateral wall")
        "WALL_TO_SEPTUM" -> Ok("Lateral wall to septum")
        _ -> Error(Nil)
      }

    tag
      if tag.group >= registry.overlay_type.tag.group
      && tag.group <= registry.overlay_type.tag.group + 0xFF
      && tag.element == registry.overlay_type.tag.element
    ->
      case value {
        "G" -> Ok("Graphics")
        "R" -> Ok("ROI")
        _ -> Error(Nil)
      }

    _ -> Error(Nil)
  }
}
