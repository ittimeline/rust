// This test makes sure that functions get annotated with the proper
// "target-cpu" attribute in LLVM.

// no-prefer-dynamic
// compile-flags: -C no-prepopulate-passes -C panic=abort -C linker-plugin-lto

#![crate_type = "staticlib"]

// CHECK-LABEL: define {{.*}} @exported() {{.*}} #0
#[no_mangle]
pub extern fn exported() {
    not_exported();
}

// CHECK-LABEL: ; target_cpu_on_functions::not_exported
// CHECK-NEXT: ; Function Attrs:
// CHECK-NEXT: define {{.*}}() {{.*}} #0
fn not_exported() {}

// CHECK: attributes #0 = {{.*}} "target-cpu"="{{.*}}"
