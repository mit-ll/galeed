// TODO - THIS FILE NEEDS TO BE AUTO-GENERATED (cbindgen?)
#include <cstdint>

using IdType = int32_t;

struct MyStruct {
    int32_t x;
    bool y;
};

struct dummy_t {}; // inheritance from empty struct has no overhead, 
                   // BUT allows us to prevent creation of new instances

template<typename T>
struct FakePtr: dummy_t {
    const IdType id;

    FakePtr() = delete;       // deleted constructors stay deleted
    FakePtr(IdType) = delete; // we can now only get these types of objects from Rust
                              // assuming we forbid casting to FakePtr
}; // if using cbindgen, can grep / sed just fix this afterwards? (cbindgen can prepend, but still need dummy inheritance)

extern "C" {
    int32_t get_x_in_MyStruct_ffi(const FakePtr<MyStruct>);
    bool get_y_in_MyStruct_ffi(const FakePtr<MyStruct>);
    void set_x_in_MyStruct_ffi(const FakePtr<MyStruct>, int32_t);
    void set_y_in_MyStruct_ffi(const FakePtr<MyStruct>, bool);
    int32_t get_field_0_in_MyStruct_ffi(const FakePtr<MyStruct>);
    bool get_field_1_in_MyStruct_ffi(const FakePtr<MyStruct>);
    void set_field_0_in_MyStruct_ffi(const FakePtr<MyStruct>, int32_t);
    void set_field_1_in_MyStruct_ffi(const FakePtr<MyStruct>, bool);
} // TODO - check, types preserved across boundary? (assuming this is auto-generated)
