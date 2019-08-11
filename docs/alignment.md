# Struct alignment in C, C++, and Rust

The RTP C++ code starts off with the following head-scratcher:

```c++
#pragma pack(push, 1)
typedef struct {
    unsigned short  flags;              // 2 bytes
    unsigned short  sequence;           // 2 bytes
    unsigned int    timestamp;          // 4 bytes
    unsigned int    ssrc;               // 4 bytes
} RTPHeader, *PRTPHeader;

typedef struct {
    unsigned short  profile_specific;
    unsigned short  ext_length;         // length of ext_data array
    unsigned int    ext_data[1];        // array length defined above
} RTPHeaderExt;
#pragma pack(pop)
```

_Short aside: `unsigned int ext_data[1]` might be a relic of the the Flexible Array Member pattern before C99_

Pragma pack is an [MSVC](https://docs.microsoft.com/en-us/cpp/preprocessor/pack?view=vs-2019) directive to specify struct alignment, which is also [supported by GCC](https://gcc.gnu.org/onlinedocs/gcc-4.4.4/gcc/Structure_002dPacking-Pragmas.html).

ESR's [struct packing article](http://www.catb.org/esr/structure-packing/) is widely shared. The code in question is an implementation of a network protocol ([RTP](https://tools.ietf.org/html/rfc3550)) - the struct packing is to obey the external standard of the RTP protocol, most likely, and not a performance optimization. ESR says:

>The only good reason for #pragma pack is if you have to exactly match your C data layout to some kind of bit-level hardware or protocol requirement, like a memory-mapped hardware port, and violating normal alignment is required for that to work.

The `push` and `pop` are a way to apply the 1-byte alignment to every struct in between the statements.

### How do I check my struct's alignment?

I asked this question on the ##C channel on Freenode and received a few answers:

* Check the struct's size with [`sizeof`](https://en.cppreference.com/w/cpp/language/sizeof)
* Check the struct's field offsets with [`offsetof`](http://man7.org/linux/man-pages/man3/offsetof.3.html)
* Check the location and address of the object
* [Pahole](https://linux.die.net/man/1/pahole)
* Print the bytes of the struct (with the following example given, and a caveat that this is only useful for debugging:)

```c
struct foo { short a; int b; long c; }
x = { 1,2,3 };
unsigned char*p=&x;
int a;
for (a=0; a < sizeof(x); a++)
    printf("%02x ",p[a]);
```

Since it matters for C, before I begin let me describe my computer and test setup:

* Lenovo Thinkpad T450s with 12GB RAM and i7 5600U
* Fedora 30 64-bit
* gcc (GCC) 9.1.1 20190503 (Red Hat 9.1.1-1)
* clang 8.0.0 (Fedora 8.0.0-1.fc30)

#### Struct with fields in descending order of size

Let's create a struct with fields in descending order of size - double,int,char i.e. 8,4,1 - and run alignment checks:

```c
#include <stdio.h>
#include <stddef.h>

struct foo_align_default {
	double x;
	int y;
	char z;
};

#pragma pack(push, 1)
struct foo_align_1 {
	double x;
	int y;
	char z;
};
#pragma pack(pop)

#pragma pack(push, 2)
struct foo_align_2 {
	double x;
	int y;
	char z;
};
#pragma pack(pop)

#pragma pack(push, 4)
struct foo_align_4 {
	double x;
	int y;
	char z;
};
#pragma pack(pop)

int main(int argc, char **argv)
{
	printf("sizeof foo, default: %lu\n", sizeof(struct foo_align_default));
	printf("sizeof foo, align 1: %lu\n", sizeof(struct foo_align_1));
	printf("sizeof foo, align 2: %lu\n", sizeof(struct foo_align_2));
	printf("sizeof foo, align 4: %lu\n", sizeof(struct foo_align_4));

	printf("offsetof foo, default, x: %lu\ty: %lu\tz: %lu\n", offsetof(struct foo_align_default, x), offsetof(struct foo_align_default, y), offsetof(struct foo_align_default, z));
	printf("offsetof foo, align 1, x: %lu\ty: %lu\tz: %lu\n", offsetof(struct foo_align_1, x), offsetof(struct foo_align_1, y), offsetof(struct foo_align_1, z));
	printf("offsetof foo, align 2, x: %lu\ty: %lu\tz: %lu\n", offsetof(struct foo_align_2, x), offsetof(struct foo_align_2, y), offsetof(struct foo_align_2, z));
	printf("offsetof foo, align 4, x: %lu\ty: %lu\tz: %lu\n", offsetof(struct foo_align_4, x), offsetof(struct foo_align_4, y), offsetof(struct foo_align_4, z));

	return 0;
}
```

Using the implicit make rules for a file named test.c which contains the above, and specifying CC=clang because the `-Wpadded` error message is better than gcc:

```
sevagh:struct-analysis $ rm test; CFLAGS="${CFLAGS} -Wpadded" CC=clang make test && ./test
clang  -Wpadded    test.c   -o test
test.c:4:8: warning: padding size of 'struct foo_align_default' with 3 bytes to alignment boundary [-Wpadded]
struct foo_align_default {
       ^
test.c:19:8: warning: padding size of 'struct foo_align_2' with 1 byte to alignment boundary [-Wpadded]
struct foo_align_2 {
       ^
test.c:27:8: warning: padding size of 'struct foo_align_4' with 3 bytes to alignment boundary [-Wpadded]
struct foo_align_4 {
       ^
3 warnings generated.
sizeof foo, default: 16
sizeof foo, align 1: 13
sizeof foo, align 2: 14
sizeof foo, align 4: 16
offsetof foo, default, x: 0     y: 8    z: 12
offsetof foo, align 1, x: 0     y: 8    z: 12
offsetof foo, align 2, x: 0     y: 8    z: 12
offsetof foo, align 4, x: 0     y: 8    z: 12
```

There's a nice illustation of struct packing and padding [here](https://katecpp.github.io/struct-members-order/). Here are representations of the above struct packing, in the same vein, in ASCII:

```
legend: x = 8-byte double, y = 4-byte int, z = 1-byte char, P = pad

                                                          1   2      4, default 
                                                          ¦   ¦       ¦
      -----------------------------------------------------------------
      ¦ x ¦ x ¦ x ¦ x ¦ x ¦ x ¦ x ¦ x ¦ y ¦ y ¦ y ¦ y ¦ z ¦ P ¦ P ¦ P ¦
      -----------------------------------------------------------------
      ¦                               ¦               ¦   ¦   ¦   ¦   ¦
bytes 0                               8               12  13  14  15  16
```

We save 3 bytes by using 1-byte alignment, but we lose the faster memory accesses that are gained from correct alignment (refer to ESR's _The Lost Art of Structure Packing_ linked above).

#### Struct with fields in mixed order of size

Let's create a struct with fields in mixed order of size - we'll go int,double,char i.e. 4,8,1 - and run alignment checks:

```c
struct foo_align_default {
	int x;
	double y;
	char z;
};
```

Results:

```
sevagh:struct-analysis $ rm test; CFLAGS="${CFLAGS} -Wpadded" CC=clang make test && ./test
clang  -Wpadded    test.c   -o test
test.c:6:9: warning: padding struct 'struct foo_align_default' with 4
      bytes to align 'y' [-Wpadded]
        double y;
               ^
test.c:4:8: warning: padding size of 'struct foo_align_default' with 7
      bytes to alignment boundary [-Wpadded]
struct foo_align_default {
       ^
test.c:19:8: warning: padding size of 'struct foo_align_2' with 1 byte to
      alignment boundary [-Wpadded]
struct foo_align_2 {
       ^
test.c:27:8: warning: padding size of 'struct foo_align_4' with 3 bytes
      to alignment boundary [-Wpadded]
struct foo_align_4 {
       ^
4 warnings generated.
sizeof foo, default: 24
sizeof foo, align 1: 13
sizeof foo, align 2: 14
sizeof foo, align 4: 16
offsetof foo, default, x: 0     y: 8    z: 16
offsetof foo, align 1, x: 0     y: 4    z: 12
offsetof foo, align 2, x: 0     y: 4    z: 12
offsetof foo, align 4, x: 0     y: 4    z: 12
```

Here we see that there are two alignments to the default 8-byte boundary in the default alignment:

```
legend: x = 4-byte int, y = 8-byte double, z = 1-byte char

                                                      1  2     4
                                                      ¦  ¦     ¦
  --------------------------------------------------------------
  ¦ x ¦ x ¦ x ¦ x ¦ y ¦ y ¦ y ¦ y ¦ y ¦ y ¦ y ¦ y ¦ z ¦  ¦  ¦  ¦
  --------------------------------------------------------------
  ¦               ¦               ¦               ¦   ¦  ¦  ¦  ¦
b 0               4               8               12  13 14 15 16


                                                                          default
                                                                                ¦
  -------------------------------------------------------------------------------
  ¦ x ¦ x ¦ x ¦ x ¦  ¦  ¦  ¦  ¦ y ¦ y ¦ y ¦ y ¦ y ¦ y ¦ y ¦ y ¦ z ¦ ¦ ¦ ¦ ¦ ¦ ¦ ¦
  -------------------------------------------------------------------------------
  ¦               ¦           ¦               ¦               ¦                 ¦
b 0               4           8               12              16                24
```

We are wasting a lot of space by not ordering our fields thoughtfully.

Ultimately, we want to pack our RTP struct with 1-byte alignment to remove any ambiguity in implicit padding, struct ordering, etc. when sending or receiving packets with any programming language.

### RTP structs in C and C++

Let's get a baseline of the alignment and padding of the RTP structs as implemented in the reference C++ code I'm copying. The structs (shown again):

```
#pragma pack(push, 1)
typedef struct {
    unsigned short  flags;              // 2 bytes
    unsigned short  sequence;           // 2 bytes
    unsigned int    timestamp;          // 4 bytes
    unsigned int    ssrc;               // 4 bytes
} RTPHeader, *PRTPHeader;

typedef struct {
    unsigned short  profile_specific;
    unsigned short  ext_length;         // length of ext_data array
    unsigned int    ext_data[1];        // array length defined above
} RTPHeaderExt;
#pragma pack(pop)
```

C results (same `sizeof` and `offsetof` print statements with clang and `-Wpadded` as previously):

```
sevagh:rtp-struct-analysis $ rm test; CFLAGS="${CFLAGS} -Wpadded" CC=clang make test && ./test
clang  -Wpadded    test.c   -o test
sizeof RTPHeader: 12
offsetof RTPHeader, flags: 0    sequence: 2     timestamp: 4    ssrc: 8
sizeof RTPHeaderExt: 12
offsetof RTPHeaderExt, profile_specific: 0      ext_length: 2   ext_data: 4
```

The C++ results are thankfully the same:

```
sevagh:rtp-struct-analysis $ mv test.c test.cpp
sevagh:rtp-struct-analysis $ rm test; CFLAGS="${CFLAGS} -Wpadded" CC=clang make test && ./test
g++     test.cpp   -o test
sizeof RTPHeader: 12
offsetof RTPHeader, flags: 0    sequence: 2     timestamp: 4    ssrc: 8
sizeof RTPHeaderExt: 12
offsetof RTPHeaderExt, profile_specific: 0      ext_length: 2   ext_data: 4
```

### Structs with fields in descending and mixed order in Rust

Similar structs to the C tests, except we'll use an i8 instead of char as our 1-byte field (a char in Rust is 4 bytes):

```rust
struct FooMixedDefault {
    x: i32,
    y: f64,
    z: i8
}

struct FooDescendingDefault {
    x: f64,
    y: i32,
    z: i8
}

#[repr(C, packed(1))]
struct FooMixed1 {
    x: i32,
    y: f64,
    z: i8
}

#[repr(C, packed(1))]
struct FooDescending1 {
    x: f64,
    y: i32,
    z: i8
}

#[repr(C, packed(2))]
struct FooMixed2 {
    x: i32,
    y: f64,
    z: i8
}

#[repr(C, packed(2))]
struct FooDescending2 {
    x: f64,
    y: i32,
    z: i8
}

#[repr(C, packed(4))]
struct FooMixed4 {
    x: i32,
    y: f64,
    z: i8
}

#[repr(C, packed(4))]
struct FooDescending4 {
    x: f64,
    y: i32,
    z: i8
}

#[repr(C, packed(8))]
struct FooMixed8 {
    x: i32,
    y: f64,
    z: i8
}

#[repr(C, packed(8))]
struct FooDescending8 {
    x: f64,
    y: i32,
    z: i8
}
```

```
sevagh:jitters-common $ cargo test -- --nocapture
offset_of FooDescendingDefault: x: 0    y: 8    z: 12
offset_of FooMixedDefault:      x: 8    y: 0    z: 12
offset_of FooDescending1:       x: 0    y: 8    z: 12
offset_of FooMixed1:            x: 0    y: 4    z: 12
offset_of FooDescending2:       x: 0    y: 8    z: 12
offset_of FooMixed2:            x: 0    y: 4    z: 12
offset_of FooDescending4:       x: 0    y: 8    z: 12
offset_of FooMixed4:            x: 0    y: 4    z: 12
offset_of FooDescending8:       x: 0    y: 8    z: 12
offset_of FooMixed8:            x: 0    y: 8    z: 16
test tests::print_offset_of ... ok
size_of FooDescendingDefault: 16
size_of FooMixedDefault:      16
size_of FooDescending1:       13
size_of FooMixed1:            13
size_of FooDescending2:       14
size_of FooMixed2:            14
size_of FooDescending4:       16
size_of FooMixed4:            16
size_of FooDescending8:       16
size_of FooMixed8:            24
test tests::print_size_of ... ok
```

It looks like Rust [automatically arranged the struct fields](https://doc.rust-lang.org/reference/type-layout.html#the-default-representation) in the `*Default*` case:

>There are no guarantees of data layout made by this representation.

As such, the default alignment of the mixed-order field struct is less bad in Rust than C - 16 vs. 24. The real result that I wanted, and I feel good enough about, is that `#[repr(C, packed(1))]` seems to be aligned in memory just like the C struct defined with `pragma pack(push, 1)`.

### RTP structs in Rust

Here's my first try of implementing the RTP structs in Rust:

```rust
struct RTPHeader {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

struct RTPHeaderExt {
    profile_specific: u16,
    ext_length: u16,
    ext_data: [u32]
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use super::*;
    use memoffset::offset_of;

    #[test]
    fn print_size_of() {
        println!("size_of RTPHeader: {:#?}", size_of::<RTPHeader>());
        //println!("size_of RTPHeaderExt: {:#?}", size_of::<RTPHeaderExt>());
    }

    #[test]
    fn print_offset_of() {
        println!("offset_of RTPHeader: flags: {:#?}\tsequence: {:#?}\ttimestamp: {:#?}\tssrc: {:#?}", offset_of!(RTPHeader, flags), offset_of!(RTPHeader, sequence), offset_of!(RTPHeader, timestamp), offset_of!(RTPHeader, ssrc));
        //println!("offset_of RTPHeaderExt: profile_specific: {:#?}\text_length: {:#?}\text_data: {:#?}", offset_of!(RTPHeaderExt, profile_specific), offset_of!(RTPHeaderExt, ext_length), offset_of!(RTPHeaderExt, ext_data));
    }
}
```

We can't play around with RTPHeaderExt because it has a dynamically sized type (the [idiomatic Rust approach for C's flexible array members](https://users.rust-lang.org/t/emulating-c-flexible-array-members-in-rust/6262)), so we'll focus on `RTPHeader`:

```
sevagh:jitters-common $ cargo test -- --nocapture
running 2 tests
size_of RTPHeader: 12
offset_of RTPHeader: flags: 8   sequence: 10    timestamp: 0    ssrc: 4
test tests::print_size_of ... ok
test tests::print_offset_of ... ok
```

The size looks to be the same as the C code, but the offsets are different. Now let's try `repr(C)`:

```rust
#[repr(C)]
struct RTPHeader {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}
```

```
sevagh:jitters-common $ cargo test -- --nocapture
running 2 tests
size_of RTPHeader: 12
test tests::print_size_of ... ok
offset_of RTPHeader: flags: 0   sequence: 2     timestamp: 4    ssrc: 8
test tests::print_offset_of ... ok
```

Finally, let's try some packing:


```rust
#[repr(C, packed(1))]
struct RTPHeader1 {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

#[repr(C, packed(2))]
struct RTPHeader2 {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

#[repr(C, packed(4))]
struct RTPHeader4 {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

#[repr(C, packed(8))]
struct RTPHeader8 {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}
```

```
sevagh:jitters-common $ cargo test -- --nocapture
running 2 tests
offset_of RTPHeader1: flags: 0  sequence: 2     timestamp: 4    ssrc: 8
offset_of RTPHeader2: flags: 0  sequence: 2     timestamp: 4    ssrc: 8
offset_of RTPHeader4: flags: 0  sequence: 2     timestamp: 4    ssrc: 8
offset_of RTPHeader8: flags: 0  sequence: 2     timestamp: 4    ssrc: 8
test tests::print_offset_of ... ok
size_of RTPHeader1: 12
size_of RTPHeader2: 12
size_of RTPHeader4: 12
size_of RTPHeader8: 16
```

### Conclusion

We found a Rust representation of the C++ RTP structs that matches the original in memory alignment:

```rust
#[repr(C, align(1))]
struct RTPHeader {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

#[repr(C, align(1))]
struct RTPHeaderExt {
    profile_specific: u16,
    ext_length: u16,
    ext_data: [u32],
}
```

This is a good starting point for implementing the rest of the RTP jitter buffer.
