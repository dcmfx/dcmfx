// This file contains the C entry point called from Rust to perform decoding of
// JPEG 2000 data.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <openjpeg.h>

static const uint8_t JP2_RFC3745_MAGIC[] = {0x00, 0x00, 0x00, 0x0c, 0x6a, 0x50, 0x20, 0x20, 0x0d, 0x0a, 0x87, 0x0a};
static const uint8_t JP2_MAGIC[] = {0x0d, 0x0a, 0x87, 0x0a};
static const uint8_t J2K_CODESTREAM_MAGIC[] = {0xff, 0x4f, 0xff, 0x51};

static void info_handler(char const *msg, void *unused)
{
#ifndef __wasm__
  fprintf(stdout, "openjpeg [I]: %s", msg);
#endif
}

static void warning_handler(char const *msg, void *unused)
{
#ifndef __wasm__
  fprintf(stdout, "openjpeg [W]: %s", msg);
#endif
}

static void error_handler(char const *msg, void *unused)
{
#ifndef __wasm__
  fprintf(stderr, "openjpeg [E]: %s", msg);
#endif
}

typedef struct
{
  uint8_t *data;
  uint64_t data_length;
  uint64_t offset;
} openjpeg_data_source;

size_t stream_read(void *p_buffer, size_t n_bytes, void *p_user_data)
{
  openjpeg_data_source *data_source = (openjpeg_data_source *)p_user_data;

  if (n_bytes == 0 || data_source->offset >= data_source->data_length)
    return -1;

  size_t remaining_data = data_source->data_length - data_source->offset;
  size_t read_length = n_bytes <= remaining_data ? n_bytes : remaining_data;
  memcpy(p_buffer, data_source->data + data_source->offset, read_length);

  data_source->offset += read_length;

  return read_length;
}

OPJ_OFF_T stream_skip(OPJ_OFF_T n_bytes, void *p_user_data)
{
  openjpeg_data_source *data_source = (openjpeg_data_source *)p_user_data;

  int64_t original_offset = (int64_t)data_source->offset;
  int64_t new_offset = original_offset + n_bytes;

  if (new_offset < 0)
    new_offset = 0;
  else if (new_offset > data_source->data_length)
    new_offset = data_source->data_length;

  data_source->offset = new_offset;

  return new_offset - original_offset;
}

OPJ_BOOL stream_seek(OPJ_OFF_T n_bytes, void *p_user_data)
{
  openjpeg_data_source *data_source = (openjpeg_data_source *)p_user_data;

  if (n_bytes < 0)
    return OPJ_FALSE;
  else if (n_bytes > (int64_t)data_source->data_length)
    n_bytes = (int64_t)data_source->data_length;

  data_source->offset = n_bytes;

  return OPJ_TRUE;
}

static void cleanup(opj_stream_t stream, opj_codec_t *codec, opj_image_t *image)
{
  opj_stream_destroy(stream);
  opj_destroy_codec(codec);
  opj_image_destroy(image);
}

int openjpeg_decode(uint8_t *input_data, uint64_t input_data_size,
                    uint32_t width, uint32_t height, uint32_t samples_per_pixel,
                    uint32_t bits_allocated, uint32_t pixel_representation,
                    uint8_t *output_data,
                    uint64_t output_data_size,
                    int8_t error_message[256])
{
  if (input_data_size < 12)
  {
    strcpy(error_message, "Input data is too small");
    return -1;
  }

  int codec_format = OPJ_CODEC_UNKNOWN;
  if (memcmp(input_data, JP2_RFC3745_MAGIC, 12) == 0 ||
      memcmp(input_data, JP2_MAGIC, 4) == 0)
    codec_format = OPJ_CODEC_JP2;
  else if (memcmp(input_data, J2K_CODESTREAM_MAGIC, 4) == 0)
    codec_format = OPJ_CODEC_J2K;
  else
  {
    strcpy(error_message, "Invalid header, not valid JPEG 2000 data");
    return -1;
  }

  opj_codec_t *codec = opj_create_decompress(codec_format);
  if (codec == NULL)
  {
    strcpy(error_message, "Failed creating decompression structure");
    return -1;
  }

  // These handlers can be enabled during development to see detailed logging
  // opj_set_info_handler(codec, info_handler, NULL);
  // opj_set_warning_handler(codec, warning_handler, NULL);
  // opj_set_error_handler(codec, error_handler, NULL);

  opj_dparameters_t parameters;
  opj_set_default_decoder_parameters(&parameters);
  if (!opj_setup_decoder(codec, &parameters))
  {
    strcpy(error_message, "Failed setting up decompressor");
    cleanup(NULL, codec, NULL);
    return -1;
  }

  opj_stream_t *stream = opj_stream_create(OPJ_J2K_STREAM_CHUNK_SIZE, 1);
  if (stream == NULL)
  {
    strcpy(error_message, "Failed setting up stream");
    cleanup(stream, codec, NULL);
    return -1;
  }

  openjpeg_data_source data_source = {input_data, input_data_size, 0};
  opj_stream_set_user_data(stream, &data_source, NULL);
  opj_stream_set_user_data_length(stream, input_data_size);
  opj_stream_set_read_function(stream, stream_read);
  opj_stream_set_skip_function(stream, stream_skip);
  opj_stream_set_seek_function(stream, stream_seek);

  opj_image_t *image = NULL;
  if (!opj_read_header(stream, codec, &image))
  {
    strcpy(error_message, "Failed reading header");
    cleanup(stream, codec, image);
    return -1;
  }

  if (image->x1 != width || image->y1 != height || image->numcomps != samples_per_pixel)
  {
    strcpy(error_message, "Image does not have the expected width, height, or samples per pixel");
    cleanup(stream, codec, image);
    return -1;
  }

  for (int i = 0; i < image->numcomps; i++)
  {
    if (image->comps[i].prec != bits_allocated || image->comps[i].sgnd != pixel_representation)
    {
      strcpy(error_message, "Image does not have the expected bits allocated or pixel representation");
      cleanup(stream, codec, image);
      return -1;
    }
  }

  if (!(opj_decode(codec, stream, image) &&
        opj_end_decompress(codec, stream)))
  {
    strcpy(error_message, "Decode failed");
    cleanup(stream, codec, image);
    return -1;
  }

  if (image->numcomps == 1)
  {
    if (image->comps[0].prec == 8)
    {
      if (output_data_size != width * height)
      {
        strcpy(error_message, "Output data is not the expected size");
        cleanup(stream, codec, image);
        return -1;
      }

      if (pixel_representation == 0)
        for (int i = 0; i < width * height; i++)
          output_data[i] = image->comps[0].data[i];
      else
        for (int i = 0; i < width * height; i++)
          ((int8_t *)output_data)[i] = image->comps[0].data[i];
    }
    else if (image->comps[0].prec == 16)
    {
      if (output_data_size != width * height * 2)
      {
        strcpy(error_message, "Output data is not the expected size");
        cleanup(stream, codec, image);
        return -1;
      }

      if (pixel_representation == 0)
        for (int i = 0; i < width * height; i++)
          ((uint16_t *)output_data)[i] = image->comps[0].data[i];
      else
        for (int i = 0; i < width * height; i++)
          ((int16_t *)output_data)[i] = image->comps[0].data[i];
    }
    else if (image->comps[0].prec == 32)
    {
      if (output_data_size != width * height * 4)
      {
        strcpy(error_message, "Output data is not the expected size");
        cleanup(stream, codec, image);
        return -1;
      }

      if (pixel_representation == 0)
        for (int i = 0; i < width * height; i++)
          ((uint32_t *)output_data)[i] = image->comps[0].data[i];
      else
        for (int i = 0; i < width * height; i++)
          ((int32_t *)output_data)[i] = image->comps[0].data[i];
    }
    else
    {
      strcpy(error_message, "Precision not supported");
      cleanup(stream, codec, image);
      return -1;
    }
  }
  else if (image->numcomps == 3)
  {
    OPJ_INT32 *red_data = image->comps[0].data;
    OPJ_INT32 *green_data = image->comps[1].data;
    OPJ_INT32 *blue_data = image->comps[2].data;

    if (image->comps[0].prec == 8)
    {
      if (output_data_size != width * height * 3)
      {
        strcpy(error_message, "Output data is not the expected size");
        cleanup(stream, codec, image);
        return -1;
      }

      for (int i = 0; i < width * height; i++)
      {
        output_data[i * 3] = red_data[i];
        output_data[i * 3 + 1] = green_data[i];
        output_data[i * 3 + 2] = blue_data[i];
      }
    }
    else if (image->comps[0].prec == 16)
    {
      if (output_data_size != width * height * 2 * 3)
      {
        strcpy(error_message, "Output data is not the expected size");
        cleanup(stream, codec, image);
        return -1;
      }

      for (int i = 0; i < width * height; i++)
      {
        ((uint16_t *)output_data)[i * 3] = red_data[i];
        ((uint16_t *)output_data)[i * 3 + 1] = green_data[i];
        ((uint16_t *)output_data)[i * 3 + 2] = blue_data[i];
      }
    }
    else if (image->comps[0].prec == 32)
    {
      if (output_data_size != width * height * 4 * 3)
      {
        strcpy(error_message, "Output data is not the expected size");
        cleanup(stream, codec, image);
        return -1;
      }

      for (int i = 0; i < width * height; i++)
      {
        ((uint32_t *)output_data)[i * 3] = red_data[i];
        ((uint32_t *)output_data)[i * 3 + 1] = green_data[i];
        ((uint32_t *)output_data)[i * 3 + 2] = blue_data[i];
      }
    }
    else
    {
      strcpy(error_message, "Precision not supported");
      cleanup(stream, codec, image);
      return -1;
    }
  }
  else
  {
    strcpy(error_message, "Number of components not supported");
    cleanup(stream, codec, image);
    return -1;
  }

  cleanup(stream, codec, image);

  return 0;
}
