// This file contains the C entry point called from Rust to perform JPEG XL
// encoding with libjxl.

#include <jxl/decode.h>
#include <jxl/encode.h>
#include <jxl/thread_parallel_runner.h>
#include <stddef.h>
#include <stdexcept>
#include <string.h>
#include <vector>

extern "C" size_t libjxl_decode(const void *input_data, size_t input_data_size,
                                size_t width, size_t height,
                                size_t samples_per_pixel, size_t bits_allocated,
                                void *output_buffer, size_t output_buffer_size,
                                char *error_buffer, size_t error_buffer_size) {
  JxlDecoder *decoder = nullptr;
  void *runner = nullptr;

  try {
    // Create decoder
    decoder = JxlDecoderCreate(nullptr);
    if (decoder == nullptr) {
      throw std::runtime_error("JxlDecoderCreate() failed");
    }

    auto status = JxlDecoderSubscribeEvents(decoder, JXL_DEC_BASIC_INFO |
                                                         JXL_DEC_FULL_IMAGE);
    if (status != JXL_DEC_SUCCESS) {
      throw std::runtime_error("JxlDecoderSubscribeEvents() failed");
    }

    // Setup parallel runner
    runner = JxlThreadParallelRunnerCreate(
        nullptr, JxlThreadParallelRunnerDefaultNumWorkerThreads());
    if (runner == nullptr) {
      throw std::runtime_error("JxlThreadParallelRunnerCreate() failed");
    }
    status =
        JxlDecoderSetParallelRunner(decoder, JxlThreadParallelRunner, runner);
    if (status != JXL_DEC_SUCCESS) {
      throw std::runtime_error("JxlEncoderSetParallelRunner() failed");
    }

    // Set input data
    status = JxlDecoderSetInput(decoder,
                                reinterpret_cast<const uint8_t *>(input_data),
                                input_data_size);
    if (status != JXL_DEC_SUCCESS) {
      throw std::runtime_error("JxlDecoderSetInput() failed");
    }

    JxlDecoderCloseInput(decoder);

    // Process input
    while (1) {
      status = JxlDecoderProcessInput(decoder);

      if (status == JXL_DEC_ERROR) {
        throw std::runtime_error("JxlDecoderProcessInput() failed");
      } else if (status == JXL_DEC_NEED_MORE_INPUT) {
        throw std::runtime_error("JPEG XL data is incomplete");
      } else if (status == JXL_DEC_BASIC_INFO) {
        // Check image dimensions
        auto info = JxlBasicInfo();
        status = JxlDecoderGetBasicInfo(decoder, &info);
        if (status != JXL_DEC_SUCCESS) {
          throw std::runtime_error("JxlDecoderImageOutBufferSize() failed");
        }

        if (info.xsize != width || info.ysize != height ||
            info.num_color_channels != samples_per_pixel) {
          throw std::runtime_error("Image does not have the expected "
                                   "dimensions or samples per pixel");
        }
      } else if (status == JXL_DEC_NEED_IMAGE_OUT_BUFFER) {
        // Construct pixel format
        auto data_type =
            bits_allocated == 16 ? JXL_TYPE_UINT16 : JXL_TYPE_UINT8;
        JxlPixelFormat format = {(uint32_t)samples_per_pixel, data_type,
                                 JXL_NATIVE_ENDIAN, 0};

        // Check output buffer size matches
        size_t expected_out_buffer_size = 0;
        status = JxlDecoderImageOutBufferSize(decoder, &format,
                                              &expected_out_buffer_size);
        if (status != JXL_DEC_SUCCESS) {
          throw std::runtime_error("JxlDecoderImageOutBufferSize() failed");
        }
        if (output_buffer_size != expected_out_buffer_size) {
          throw std::runtime_error("Incorrect output buffer size");
        }

        // Set output buffer
        status = JxlDecoderSetImageOutBuffer(decoder, &format, output_buffer,
                                             output_buffer_size);
        if (status != JXL_DEC_SUCCESS) {
          throw std::runtime_error("JxlDecoderSetImageOutBuffer() failed");
        }
      } else if (status == JXL_DEC_FULL_IMAGE || status == JXL_DEC_SUCCESS) {
        break;
      }
    }

    JxlDecoderDestroy(decoder);
    JxlThreadParallelRunnerDestroy(runner);

    return 0;
  } catch (const std::runtime_error &e) {
    snprintf(error_buffer, error_buffer_size, "%s", e.what());

    JxlDecoderDestroy(decoder);
    JxlThreadParallelRunnerDestroy(runner);

    return 1;
  }
}

extern "C" size_t
libjxl_encode(const void *input_data, size_t input_data_size, size_t width,
              size_t height, size_t samples_per_pixel, size_t bits_allocated,
              size_t is_color, size_t lossless, size_t quality, size_t effort,
              void *(*output_data_callback)(size_t new_len, void *ctx),
              void *output_data_context, char *error_buffer,
              size_t error_buffer_size) {
  JxlEncoder *encoder = nullptr;
  void *runner = nullptr;

  try {
    // Create encoder
    encoder = JxlEncoderCreate(nullptr);
    if (encoder == nullptr) {
      throw std::runtime_error("JxlEncoderCreate()");
    }

    // Setup parallel runner
    runner = JxlThreadParallelRunnerCreate(
        nullptr, JxlThreadParallelRunnerDefaultNumWorkerThreads());
    if (runner == nullptr) {
      throw std::runtime_error("JxlThreadParallelRunnerCreate()");
    }
    auto status =
        JxlEncoderSetParallelRunner(encoder, JxlThreadParallelRunner, runner);
    if (status != JXL_ENC_SUCCESS) {
      throw std::runtime_error("JxlEncoderSetParallelRunner()");
    }

    // Set basic image info
    auto basic_info = JxlBasicInfo();
    JxlEncoderInitBasicInfo(&basic_info);
    basic_info.xsize = width;
    basic_info.ysize = height;
    basic_info.bits_per_sample = bits_allocated;
    basic_info.num_color_channels = samples_per_pixel;

    if (lossless) {
      basic_info.uses_original_profile = 1;
    }

    status = JxlEncoderSetBasicInfo(encoder, &basic_info);
    if (status != JXL_ENC_SUCCESS) {
      throw std::runtime_error("JxlEncoderSetBasicInfo()");
    }

    // Set input color encoding
    auto color_encoding = JxlColorEncoding();
    JxlColorEncodingSetToSRGB(&color_encoding, !is_color);
    status = JxlEncoderSetColorEncoding(encoder, &color_encoding);
    if (status != JXL_ENC_SUCCESS) {
      throw std::runtime_error("JxlEncoderSetColorEncoding()");
    }

    // Determine input data type
    auto data_type = bits_allocated == 16 ? JXL_TYPE_UINT16 : JXL_TYPE_UINT8;

    // Set pixel format
    auto pixel_format = JxlPixelFormat{(uint32_t)samples_per_pixel, data_type,
                                       JXL_NATIVE_ENDIAN, 0};

    // Create frame settings
    auto frame_settings = JxlEncoderFrameSettingsCreate(encoder, nullptr);

    // Setup for lossy/lossless encoding
    if (lossless) {
      status = JxlEncoderSetFrameLossless(frame_settings, JXL_TRUE);
      if (status != JXL_ENC_SUCCESS) {
        throw std::runtime_error("JxlEncoderSetFrameLossless()");
      }
    } else {
      auto distance = JxlEncoderDistanceFromQuality(quality);

      status = JxlEncoderSetFrameDistance(frame_settings, distance);
      if (status != JXL_ENC_SUCCESS) {
        throw std::runtime_error("JxlEncoderSetFrameDistance()");
      }

      // Use XYB for lossy color images
      if (is_color) {
        status = JxlEncoderFrameSettingsSetOption(
            frame_settings, JXL_ENC_FRAME_SETTING_MODULAR, 0);
        if (status != JXL_ENC_SUCCESS) {
          throw std::runtime_error("JxlEncoderFrameSettingsSetOption()");
        }
      }
    }

    // Apply compression effort setting
    status = JxlEncoderFrameSettingsSetOption(
        frame_settings, JXL_ENC_FRAME_SETTING_EFFORT, effort);
    if (status != JXL_ENC_SUCCESS) {
      throw std::runtime_error("JxlEncoderFrameSettingsSetOption()");
    }

    // Provide pixel data to the encoder
    status = JxlEncoderAddImageFrame(frame_settings, &pixel_format, input_data,
                                     input_data_size);
    if (status != JXL_ENC_SUCCESS) {
      throw std::runtime_error("JxlEncoderAddImageFrame()");
    }

    JxlEncoderCloseInput(encoder);

    // Perform encoding and collect output
    size_t output_size = 0;
    status = JXL_ENC_NEED_MORE_OUTPUT;
    while (status == JXL_ENC_NEED_MORE_OUTPUT) {
      size_t output_chunk_size = 512 * 1024;

      auto initial_size = output_size;

      output_size += output_chunk_size;
      auto output_data = output_data_callback(output_size, output_data_context);

      auto next_out = reinterpret_cast<uint8_t *>(output_data) + initial_size;
      auto avail_out = output_chunk_size;

      status = JxlEncoderProcessOutput(encoder, &next_out, &avail_out);

      output_size -= avail_out;
      output_data_callback(output_size, output_data_context);
    }

    if (status == JXL_ENC_ERROR) {
      throw std::runtime_error("JxlEncoderProcessOutput()");
    }

    JxlEncoderDestroy(encoder);
    JxlThreadParallelRunnerDestroy(runner);

    return 0;
  } catch (const std::runtime_error &e) {
    snprintf(error_buffer, error_buffer_size, "%s failed with %i", e.what(),
             JxlEncoderGetError(encoder));

    JxlEncoderDestroy(encoder);
    JxlThreadParallelRunnerDestroy(runner);

    return 1;
  }

  return 0;
}
