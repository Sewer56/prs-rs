# About

`prs_rs` is an acceptably fast, barebones implementation of the SEGA PRS encoding scheme.

It can compress, decompress and calculate the decompressed size of PRS encoded data.

## Usage

=== "Rust"

    !!! warning "Requires Nightly Rust, and Unsafe"

    !!! note "We use `new_uninit` APIs to ensure we don't unnecessarily zero fill arrays we will immediately replace."

    ### Compress Data
    
    ```rust
    let src: &[u8] = b"Your data here";

    // Calculate max buffer size needed for compression
    let max_comp_len = prs_calculate_max_compressed_size(src_len);

    // Allocate enough memory for the compressed data
    let mut dest = Box::<[u8]>::new_uninit_slice(max_comp_len); 
    let bytes_written = prs_compress_unsafe(src.as_ptr(), src.len(), dest.as_mut_ptr() as *mut u8);

    // Tell Rust our memory is initialized and trim the returned slice.
    let dest = dest.assume_init(); 
    let compressed_data: &[u8] = &dest[..bytes_written];
    ```

    This API accepts slices or raw pointers as `destination`.

    ### Decompress Data

    ```rust
    let compressed_data: &[u8] = &[]; // some data
    let mut decompressed_data = Box::<[u8]>::new_uninit_slice(decompressed_len);
    let decompressed_size = prs_rs::decomp::prs_decompress_unsafe(compressed_data.as_ptr(), decompressed_data.as_mut_ptr() as *mut u8);

    let dest = decompressed_data.assume_init();
    ```

    ### Calculate Decompressed Size

    If you need to calculate the size of the decompressed data without actually decompressing it:

    ```rust
    unsafe {
        let compressed_data: &[u8] = &[];
        let decompressed_size = prs_calculate_decompressed_size(compressed_data.as_ptr());
    }
    ```

=== "C"

    !!! info "You can get header, static libraries and dynamic libraries in the [Releases](https://github.com/Sewer56/prs-rs/releases) section."

    !!! note "AI Generated, if this is inaccurate, please send a PR fixing this."

    ### Compress Data

    ```c
    unsigned char src[] = "Your data here";
    size_t src_len = sizeof(src);
    size_t max_compressed_size = prs_calculate_max_compressed_size(src_len);
    unsigned char* dest = (unsigned char*)malloc(max_compressed_size); // Dynamically allocate memory
    size_t bytes_written = prs_compress(src, dest, src_len);
    ```

    ### Decompress Data

    Decompressing data requires ensuring the destination buffer is adequately sized.

    ```c
    // Assuming `compressed_data` are available
    unsigned char* compressed_data; // Placeholder for compressed data pointer
    
    // Can also get decompressed size from e.g. archive header.
    size_t decompressed_size = prs_calculate_decompressed_size(compressed_data);
    unsigned char* dest = (unsigned char*)malloc(decompressed_size);
    size_t actual_decompressed_size = prs_decompress(compressed_data, dest);
    ```

    ### Calculate Decompressed Size

    If you need to calculate the size of the decompressed data without actually decompressing it:

    ```c
    decompressed_size = prs_calculate_decompressed_size(compressed_data_ptr);
    ```

=== "C#"

    !!! info "Published on NuGet as [prs_rs.Net.Sys](https://www.nuget.org/packages/prs_rs.Net.Sys)."

    The .NET library only provides raw bindings to the C exports, if you want more idiomatic .NET APIs, 
    please make another library on top of this one. 

    Below are some usage examples.

    ### Compress Data

    ```csharp
    public static unsafe Span<byte> CompressData(byte[] sourceData)
    {
        fixed (byte* srcPtr = sourceData)
        {
            // Get the maximum possible size of the compressed data
            nuint maxCompressedSize = NativeMethods.prs_calculate_max_compressed_size((nuint)sourceData.Length);
            byte[] dest = GC.AllocateUninitializedArray<byte>((int)maxCompressedSize);
            fixed (byte* destPtr = &dest[0])
            {
                nuint compressedSize = NativeMethods.prs_compress(srcPtr, destPtr, (nuint)sourceData.Length);
                return dest.AsSpan(0..(int)compressedSize);
            }
        }
    }
    ```

    ### Decompress Data

    Decompressing data requires ensuring the destination buffer is adequately sized.

    ```csharp
    public static unsafe Span<byte> DecompressData(byte[] compressedData)
    {
      // Calculate the decompressed size to allocate enough memory
      fixed (byte* srcPtr = compressedData)
      {
          // or get from file header etc.
          nuint decompressedSize = NativeMethods.prs_calculate_decompressed_size(srcPtr); 
          byte[] dest = GC.AllocateUninitializedArray<byte>((int)decompressedSize);
          fixed (byte* destPtr = &dest[0])
          {
              nuint actualDecompressedSize = NativeMethods.prs_decompress(srcPtr, destPtr);
              return dest.AsSpan(0..(int)decompressedSize);
          }
      }
    }
    ```

    ### Calculate Decompressed Size

    If you need to calculate the size of the decompressed data without actually decompressing it:

    ```csharp
    public static unsafe nuint DecompressData(byte[] compressedData)
    {
      // Calculate the decompressed size to allocate enough memory
      fixed (byte* srcPtr = compressedData)
      {
          // or get from file header etc.
          return NativeMethods.prs_calculate_decompressed_size(srcPtr); 
      }
    }
    ```

## Reference Performance Numbers

!!! info "System Info"

    - Library Version: 0.1.0 (07 Feb 2024)
    - CPU: AMD Ryzen 9 5900X (12C/24T)
    - RAM: 32GB DDR4-3000 (16-17-17-35)
    - OS: Archlinux

!!! note "The following reference numbers are based on PGO optimized builds of the library."
  
!!! warning "Performance numbers greatly depend on nature of input data. Data with long repeating padding compresses quicker."

All numbers are for single threaded operations.

Estimate (Determine size of data to decompress):

- `1.4`-`8.2` GiB/s.

Decompress (Decompress):

- `0.89`-`1.9` GiB/s

Compress:

- `16`-`108` MiB/s (Average: ~30MiB/s)

!!! warning "Compression uses `4*FileSize` amount of RAM"

I'm not a compression expert, I just used some brain cells and hope to have made a solution that is 
'good enough'. The compression is also optimal, i.e. it will generate the smallest possible PRS file for the given input data.

## Technical Questions

If you have questions/bug reports/etc. feel free to [Open an Issue](https://github.com/Sewer56/prs-rs/issues).

Happy Documenting ❤️