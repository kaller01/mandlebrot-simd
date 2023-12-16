# Mandlebrot with SIMD instructions in Rust
Sequentially, mandlebrot usually looks like this:
```rust
fn mandlebrot_simd(){
    for y in 0..height {
        for x in 0..width {
            let vx = x as f32;
            // Do mandlebrot iteration on vx
        }
    }
}
```

Using `packed_simd`, we can do 8 iterations at a time:
```rust
fn mandlebrot_simd(){
    for y in 0..height {
        for x in (0..width).step_by(8) {
            // Initate vx from x + 0..8
            let vx: f32x8 = f32x8::splat(x as f32)
                + unsafe {
                    f32x8::from_slice_unaligned_unchecked(&[
                        0., 1., 2., 3., 4., 5., 6., 7.,
                    ])
                };
            // Do mandlebrot iteration on vx
        }
    }
}
```
assuming the hardware supports AVX2 (256-bits register)

![mandlebrot](./mandlebrot_simd.png)

## Performance
```bash
cargo run --release
```
A speedup of about 3.3 times, very good!
```
AVX2 is supported!
Mandlebrot size of 1024x1024
SIMD: Time taken (ms): 33
SEQ: Time taken (ms): 108
```
```
AVX2 is supported!
Mandlebrot size of 4096x4096
SIMD: Time taken (ms): 512
SEQ: Time taken (ms): 1733
```
Theoretically, you should be able to achive 8 times the speedup.
Why only 3.3 times here, is due to load balancing issues of the `f32x8`. Since SIMD (single instruction multiple data), the position that iterates the longest of the 8 pixels will take the most time.   
   
For example if we have 4 lanes, where three lanes finish "instantly" and the last iterate until 256 `[0, 0, 0, 256]` we would not get any speedup from using `f32x4`, as the last iteration would keep the last lane of the `f32x4` busy. However if all lanes iterate 256 times, `[256, 256, 256, 256]` we would get a 4 times speedup.