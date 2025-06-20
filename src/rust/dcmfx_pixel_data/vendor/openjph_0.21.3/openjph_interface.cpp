// This file contains the C entry points called from Rust to perform
// High-Throughput JPEG 2000 encoding with OpenJPH.

#include <stdexcept>
#include <vector>

#include "./src/common/ojph_base.h"
#include "./src/common/ojph_codestream.h"
#include "./src/common/ojph_file.h"
#include "./src/common/ojph_mem.h"
#include "./src/common/ojph_params.h"

// Callback function that receives compressed output bytes
typedef void (*output_data_callback_t)(const void *data, uint32_t len,
                                       void *ctx);

// Outfile implementation that writes all bytes through to an output data
// callback provided by the Rust code
class callback_outfile : public ojph::outfile_base {
public:
  callback_outfile(output_data_callback_t output_data_callback,
                   void *output_data_context) {
    this->output_data_callback = output_data_callback;
    this->output_data_context = output_data_context;
  }

  virtual ~callback_outfile() override {}

  virtual size_t write(const void *ptr, size_t size) override {
    this->output_data_callback(ptr, size, this->output_data_context);
    return size;
  }

private:
  output_data_callback_t output_data_callback;
  void *output_data_context;
};

template <typename T> void fill_lines(ojph::codestream &cs, const T *in);

extern "C" size_t openjph_encode(
    const void *input_data, size_t width, size_t height,
    size_t samples_per_pixel, size_t bits_allocated, size_t bits_stored,
    size_t pixel_representation, size_t color_photometric_interpretation,
    float quantization_step_size, output_data_callback_t output_data_callback,
    void *output_data_context, char *error_buffer, size_t error_buffer_size) {

  try {
    auto cs = ojph::codestream();

    // Set image extents
    cs.access_siz().set_image_extent(ojph::point(width, height));

    // Setup image components
    auto downsampling = ojph::point(1, 1);
    auto is_signed = pixel_representation == 1;
    cs.access_siz().set_num_components(samples_per_pixel);
    for (size_t i = 0; i < samples_per_pixel; i++) {
      cs.access_siz().set_component(i, downsampling, bits_stored, is_signed);
    }

    // Enable color transform if using YBR_ICT or YBR_RCT, in which case the
    // input data will be RGB
    cs.access_cod().set_color_transform(color_photometric_interpretation == 3 ||
                                        color_photometric_interpretation == 4);

    // Setup encoding parameters for lossy/lossless
    cs.set_planar(quantization_step_size == 0.0 &&
                  color_photometric_interpretation != 3 &&
                  color_photometric_interpretation != 4);
    cs.access_cod().set_reversible(quantization_step_size == 0.0);
    if (quantization_step_size != 0.0) {
      cs.access_qcd().set_irrev_quant(quantization_step_size);
    }

    // Create outfile that sends data straight to the output callback
    auto outfile = callback_outfile(output_data_callback, output_data_context);

    // Write headers
    cs.write_headers(&outfile);

    // Fill the lines of input data
    if (bits_allocated == 8) {
      if (pixel_representation == 0) {
        fill_lines(cs, reinterpret_cast<const uint8_t *>(input_data));
      } else {
        fill_lines(cs, reinterpret_cast<const int8_t *>(input_data));
      }
    } else if (bits_allocated == 16) {
      if (pixel_representation == 0) {
        fill_lines(cs, reinterpret_cast<const uint16_t *>(input_data));
      } else {
        fill_lines(cs, reinterpret_cast<const int16_t *>(input_data));
      }
    } else if (bits_allocated == 32) {
      if (pixel_representation == 0) {
        fill_lines(cs, reinterpret_cast<const uint32_t *>(input_data));
      } else {
        fill_lines(cs, reinterpret_cast<const int32_t *>(input_data));
      }
    } else {
      throw std::runtime_error("Bits allocated value not supported");
    }

    cs.flush();

    return 0;
  } catch (const std::runtime_error &e) {
    strncpy(error_buffer, e.what(), error_buffer_size - 1);
    return 1;
  }
}

template <typename T> void fill_lines(ojph::codestream &cs, const T *in) {
  auto width = cs.access_siz().get_image_extent().x;
  auto samples_per_pixel = cs.access_siz().get_num_components();

  auto component_index = uint32_t();
  ojph::line_buf *line = nullptr;

  auto component_y_positions = std::vector<int>(samples_per_pixel, 0);

  while ((line = cs.exchange(line, component_index)) != nullptr) {
    auto &y = component_y_positions[component_index];

    for (size_t x = 0; x < width; ++x) {
      line->i32[x] = in[(y * width + x) * samples_per_pixel + component_index];
    }

    y++;
  }
}
