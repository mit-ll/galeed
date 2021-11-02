; NOTE: Assertions have been autogenerated by utils/update_test_checks.py UTC_ARGS: --function-signature --scrub-attributes
; RUN: opt -S -basicaa -attributor -attributor-disable=false -attributor-max-iterations-verify -attributor-max-iterations=3 < %s | FileCheck %s --check-prefixes=CHECK,OLDPM_MODULE
; RUN: opt -S -passes='attributor' -aa-pipeline='basic-aa' -attributor-disable=false -attributor-max-iterations-verify -attributor-max-iterations=3 < %s | FileCheck %s --check-prefixes=CHECK,NEWPM_MODULE

; OLDPM_MODULE-NOT: @dead
; NEWPM_MODULE-NOT: @dead

define internal void @dead() {
  call i32 @test(i32* null, i32* null)
  ret void
}

define internal i32 @test(i32* %X, i32* %Y) {
; CHECK-LABEL: define {{[^@]+}}@test()
; CHECK-NEXT:    br i1 true, label [[LIVE:%.*]], label [[DEAD:%.*]]
; CHECK:       live:
; CHECK-NEXT:    ret i32 0
; CHECK:       dead:
; CHECK-NEXT:    unreachable
;
  br i1 true, label %live, label %dead
live:
  ret i32 0
dead:
  call i32 @caller(i32* null)
  call void @dead()
  ret i32 1
}

define internal i32 @caller(i32* %B) {
; CHECK-LABEL: define {{[^@]+}}@caller()
; CHECK-NEXT:    [[A:%.*]] = alloca i32
; CHECK-NEXT:    store i32 1, i32* [[A]], align 4
; CHECK-NEXT:    [[C:%.*]] = call i32 @test()
; CHECK-NEXT:    ret i32 0
;
  %A = alloca i32
  store i32 1, i32* %A
  %C = call i32 @test(i32* %A, i32* %B)
  ret i32 %C
}

define i32 @callercaller() {
; CHECK-LABEL: define {{[^@]+}}@callercaller()
; CHECK-NEXT:    [[B:%.*]] = alloca i32
; CHECK-NEXT:    store i32 2, i32* [[B]], align 4
; CHECK-NEXT:    [[X:%.*]] = call i32 @caller()
; CHECK-NEXT:    ret i32 0
;
  %B = alloca i32
  store i32 2, i32* %B
  %X = call i32 @caller(i32* %B)
  ret i32 %X
}
