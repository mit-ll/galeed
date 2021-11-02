#include "llvm/IR/Function.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/Type.h"
#include "llvm/IR/Value.h"
#include "llvm/IR/Verifier.h"
#include "llvm/Pass.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Transforms/IPO/PassManagerBuilder.h"
#include "llvm/Transforms/Utils/BasicBlockUtils.h"
using namespace llvm;

const int FAKEPTR_NUM_BITS = 32;
const std::string FAKEPTR_NAME = "FakePtr";

// TODO - when integrating with LLVM, give myself a true backdoor to do this
// ORRRRR build LLVM without assertions
void ReplaceAllUsesWith_Unsafe(Argument *from, Argument *to) {
  while (!from->use_empty()) {
    auto &U = *from->use_begin();
    U.set(to);
  }
  // from->eraseFromParent();
}

// TODO - when integrating with LLVM, give myself a true backdoor to do this
// ORRRRR build LLVM without assertions
void ReplaceInstWithInst_Unsafe(Instruction *from, Instruction *to) {
  while (!from->use_empty()) {
    auto &U = *from->use_begin();
    U.set(to);
  }
  from->eraseFromParent();
}

void printType(const Type* t) {
  auto id = t->getTypeID();
  switch (id) {
    case Type::TypeID::HalfTyID: errs() << "HalfTyID"; break;
    // case Type::TypeID::BFloatTyID: errs() << "BFloatTyID"; break;
    case Type::TypeID::FloatTyID: errs() << "FloatTyID"; break;
    case Type::TypeID::DoubleTyID: errs() << "DoubleTyID"; break;
    case Type::TypeID::X86_FP80TyID: errs() << "X86_FP80TyID"; break;
    case Type::TypeID::FP128TyID: errs() << "FP128TyID"; break;
    case Type::TypeID::PPC_FP128TyID: errs() << "PPC_FP128TyID"; break;
    case Type::TypeID::VoidTyID: errs() << "VoidTyID"; break;
    case Type::TypeID::LabelTyID: errs() << "LabelTyID"; break;
    case Type::TypeID::MetadataTyID: errs() << "MetadataTyID"; break;
    case Type::TypeID::X86_MMXTyID: errs() << "X86_MMXTyID"; break;
    case Type::TypeID::TokenTyID: errs() << "TokenTyID"; break;
    case Type::TypeID::IntegerTyID:
      {
        if (auto* int_t = dyn_cast<IntegerType>(t)) {
          errs() << "IntegerTyID (with " << int_t->getBitWidth() << " bits)"; 
          break;
        }
      }
    case Type::TypeID::FunctionTyID: errs() << "FunctionTyID"; break;
    case Type::TypeID::PointerTyID: 
      errs() << "PointerTyID (pointing to a "; 
      printType(t->getPointerElementType());
      errs() << ")";
      break;
    case Type::TypeID::StructTyID:
      errs() << "StructTyID";
      if (!t->getStructName().empty()) {
        errs() << " (with struct name: " << t->getStructName() << ")";
      }
      break;
    case Type::TypeID::ArrayTyID: errs() << "ArrayTyID"; break;
    case Type::TypeID::VectorTyID: errs() << "VectorTyID"; break;
    // case Type::TypeID::FixedVectorTyID: errs() << "FixedVectorTyID"; break;
    // case Type::TypeID::ScalableVectorTyID: errs() << "ScalableVectorTyID"; break;
    default: errs() << "not found";
  }
}

StructType* getFakePtrType(LLVMContext& ctx) {
  auto* int_t = IntegerType::get(ctx, FAKEPTR_NUM_BITS);
  auto elements = std::vector<Type*> {int_t};
  auto* struct_t = StructType::create(ctx, elements, FAKEPTR_NAME); // isPacked=false);
  return struct_t;
}

void argProbing(const Function &f) {
  for (auto& arg : f.args()) {
    errs() << "I saw argument #" << arg.getArgNo();
    if (!arg.getName().empty()) {
      errs() << " called " << arg.getName();
    }
    errs() << " with type ";
    printType(arg.getType());
    errs() << "\n";
  }
}

void blockProbing(const Function &f) {
  for (auto& block : f) {
    errs() << "Block name: ";
    block.printAsOperand(errs(), false);
    errs() << "\n";
  }
}

bool safeFuncProbing(const Function &f) {
  argProbing(f);
  blockProbing(f);
  for (auto& block : f) {
    for (auto& inst : block) {
      errs() << "  Instr: " << inst << "\n";
      // if (auto* callinst = dyn_cast<CallInst>(&inst)) {
      //   printType(callinst->getType());
      //   errs() << "\n";
      // }
    }
  }
  return false;
}

bool unsafeFuncProbing(const Function &f) {
  argProbing(f);
  blockProbing(f);
  for (auto& block : f) {
    for (auto& inst : block) {
      errs() << "  Instr: " << inst << "\n";
    }

  }
  return false;
}

std::string makeRealStructName(const std::string& structname) {
  return "struct." + structname;
}

std::string getStrippedStructName(const Type* t) {
  if (auto* struct_t = dyn_cast_or_null<StructType>(t)) {
    if (!struct_t->hasName()) return "";
    auto full_struct_name = struct_t->getName();
    if (!full_struct_name.startswith("struct.")) return full_struct_name;
    return full_struct_name.substr(7);
  } else if (auto* ptr_t = dyn_cast_or_null<PointerType>(t)) {
    return getStrippedStructName(ptr_t->getElementType());
  }
  return "";
}

bool isStructWithName(const Type* t, const std::vector<std::string>& struct_names) {
  if (auto* struct_t = dyn_cast_or_null<const StructType>(t)) {
    return (struct_t->hasName() && 
            std::find(struct_names.begin(), struct_names.end(), struct_t->getName()) != struct_names.end());
  }
  return false;
}

bool isStructPtrWithName(const Type* t, const std::vector<std::string>& struct_names) {
  if (auto* ptr_t = dyn_cast_or_null<const PointerType>(t)) {
    return isStructWithName(ptr_t->getElementType(), struct_names);
  }
  return false;
}

Type* correctType(Type* t, const std::vector<std::string>& struct_names, Type* replacement) {
  if (isStructPtrWithName(t, struct_names)) {
    return replacement;
  }
  return t;
}

StoreInst* getFirstArgStoreInst(Function& f, unsigned argi) {
  auto* arg = f.getArg(argi);
  for (auto& block : f) {
    for (auto& inst : block) {
      if (auto* storeInst = dyn_cast<StoreInst>(&inst)) {
        if (storeInst->getValueOperand() == arg) {
          return storeInst;
        }
      }
    }
  }
}

Instruction* getFirstNonAllocaInst(Function& f) {
  for (auto& block: f) {
    for (auto& inst: block) {
      if (auto* alloca_inst = dyn_cast<AllocaInst>(&inst)) continue;
      return &inst;
    }
  }
}

GetElementPtrInst* getFirstGetElemPtrToChange(Function& f, const std::vector<std::string>& struct_names) {
  for (auto& block : f) {
    for (auto& inst : block) {
      if (auto* gepInst = dyn_cast<GetElementPtrInst>(&inst)) {
        if (isStructWithName(gepInst->getSourceElementType(), struct_names) &&
            isStructPtrWithName(gepInst->getPointerOperandType(), struct_names)) {
          return gepInst;
        }
      }
    }
  }
  return nullptr;
}

void correctGetInsts(LoadInst* inst) {
  // errs() << "correctGetInsts: " << "\n";
  // errs() << "  " << *inst << "\n";
  // auto* prev_inst = inst->getPrevNonDebugInstruction();
  // errs() << "  " << *prev_inst << "\n";
  // auto* prev_prev_inst = prev_inst->getPrevNonDebugInstruction();
  // errs() << "  " << *prev_prev_inst << "\n";
}

void correctSetInsts(StoreInst* inst) {

}

namespace {
  struct FakePtrPass : public ModulePass {
    static char ID;
    FakePtrPass() : ModulePass(ID) {}

    virtual bool runOnModule(Module &M) {
      auto stub_struct_name = "MyStruct";
      const std::vector<std::string> struct_names {makeRealStructName(stub_struct_name)};

      bool isChanged = false;
      auto& ctx = M.getContext();
      auto* fakeptr_t = getFakePtrType(ctx);
      auto* fakeptr_ptr_t = PointerType::get(fakeptr_t, 0);
      auto* int32arg_t = IntegerType::get(ctx, 32); // the C abi passes transparently (I think?)
      // weird iteration to avoid problems deleting f
      for (auto f = M.begin(); f != M.end(); ) {
        auto* oldFun = &*f++;
        
        if (oldFun->getInstructionCount() == 0) {
          // declarations (including llvm intrinsics) are also included in the module, this avoids those
          continue;
        } 
        errs() << "function name: " << oldFun->getName() << "\n";


        auto* oldFunTy = oldFun->getFunctionType();
        auto oldAttributeList = oldFun->getAttributes();
  
        std::vector<Type*> params;
        std::vector<AttributeSet> newParamAttributes;
        std::vector<unsigned> changedArgs;
        unsigned argi = 0;
        for (auto& arg : oldFun->args()) {
          auto* arg_t = arg.getType();
          auto* corrected_t = correctType(arg_t, struct_names, int32arg_t); //correctType(arg_t, struct_names, fakeptr_t)
          if (arg_t != corrected_t) {
            changedArgs.push_back(argi);
          }
          params.push_back(corrected_t);
          newParamAttributes.push_back(oldAttributeList.getParamAttributes(argi++));
        }

        // TODO - in order to change return type here, we must also be prepared to change
        // the return instructions other places (maybe, not 100% sure on this one)
        auto* newRetTy = oldFunTy->getReturnType(); // correctType(oldFunTy->getReturnType(), struct_names, fakeptr_t);

        auto* newFunTy = FunctionType::get(newRetTy, params, oldFunTy->isVarArg());
        
        if (newFunTy == oldFunTy) {
          errs() << "Nothing to do! \n";

          // TODO - remove, only for debugging
          safeFuncProbing(*oldFun);

          continue;
        }

        unsafeFuncProbing(*oldFun);

        auto* newFun = Function::Create(newFunTy, oldFun->getLinkage(), oldFun->getAddressSpace());
        newFun->copyAttributesFrom(oldFun);
        newFun->setComdat(oldFun->getComdat());

        auto newAttributeList = AttributeList::get(oldFun->getContext(), 
                                                   oldAttributeList.getFnAttributes(), 
                                                   oldAttributeList.getRetAttributes(),
                                                   newParamAttributes);
        newFun->setAttributes(newAttributeList);

        oldFun->getParent()->getFunctionList().insert(oldFun->getIterator(), newFun);
        newFun->takeName(oldFun);

        newFun->getBasicBlockList().splice(newFun->begin(), oldFun->getBasicBlockList());

        for (auto oldArg = oldFun->arg_begin(), newArg = newFun->arg_begin();
             oldArg != oldFun->arg_end();
             ++oldArg, ++newArg) {
          // oldArg->replaceAllUsesWith(&*newArg);
          ReplaceAllUsesWith_Unsafe(oldArg, &*newArg);
          newArg->takeName(&*oldArg);
        }

        // copying metadata (ripped directly from example in LLVM codebase)
        SmallVector<std::pair<unsigned, MDNode *>, 1> MDs;
        oldFun->getAllMetadata(MDs);
        for (auto MD : MDs) {
          newFun->addMetadata(MD.first, *MD.second);
        }

        // Next step: correct store instructions to use FakePtrs
        // Naive approach: first store for any argument, then follow the trail back
        for (auto argi : changedArgs) {
          auto* oldStoreInst = getFirstArgStoreInst(*newFun, argi);
          if (oldStoreInst) {
            if (auto* oldAllocaInst = dyn_cast_or_null<AllocaInst>(oldStoreInst->getPointerOperand())) {
              auto* newAllocaInst = new AllocaInst(fakeptr_t, 0, nullptr, MaybeAlign(4), "arg_fakeptr_alloca", oldAllocaInst);
              // ReplaceInstWithInst(oldAllocaInst, newAllocaInst);
              ReplaceInstWithInst_Unsafe(oldAllocaInst, newAllocaInst);
              auto* constantint = ConstantInt::get(int32arg_t, 0);
              std::vector<Value*> constantarray = {constantint, constantint};
              auto arrayref = ArrayRef<Value*>(constantarray);
              auto* newGetElemPtrInst = GetElementPtrInst::CreateInBounds(newAllocaInst, arrayref, "helpme", oldStoreInst);

              auto* newStoreInst = new StoreInst(oldStoreInst->getValueOperand(), newGetElemPtrInst, false, Align(4));
              ReplaceInstWithInst(oldStoreInst, newStoreInst);
            }
          }
        }

        // Mark first non-AllocaInst to place new AllocaInsts before (for copying FakePtrs)
        auto* last_alloca_inst = getFirstNonAllocaInst(*newFun)->getPrevNonDebugInstruction();

        std::vector<LoadInst*> queued_loadinsts;
        std::vector<StoreInst*> queued_storeinsts;

        // Loop through instructions, collect LoadInsts and StoreInsts to modify
        for (auto& block: *newFun) {
          for (auto& inst: block) {
            // TODO - make these checks a separate function?
            if (auto* load_inst = dyn_cast<LoadInst>(&inst)) {
              if (auto* gep_inst = dyn_cast<GetElementPtrInst>(load_inst->getPointerOperand())) {
                if (isStructPtrWithName(gep_inst->getPointerOperandType(), struct_names)) {
                  if (auto* prev_load_inst = dyn_cast<LoadInst>(gep_inst->getPointerOperand())) {
                    if (prev_load_inst->getPointerOperandType() != fakeptr_ptr_t) {
                      errs() << "TODO - determine failure mode (probably solved by optimization)\n";
                      continue; // Should result in mismatched load type, which will then segfault??
                      // If we chain these, do we need to do multiple rounds of queueing? In fact worse, if these butt up against each other, I may have to loop every time
                    }
                    queued_loadinsts.push_back(load_inst);
                  }
                }
              }
            }
            else if (auto* store_inst = dyn_cast<StoreInst>(&inst)) {
              if (auto* gep_inst = dyn_cast<GetElementPtrInst>(store_inst->getPointerOperand())) {
                if (isStructPtrWithName(gep_inst->getPointerOperandType(), struct_names)) {
                  if (auto* prev_load_inst = dyn_cast<LoadInst>(gep_inst->getPointerOperand())) {
                    if (prev_load_inst->getPointerOperandType() != fakeptr_ptr_t) {
                      errs() << "TODO - determine failure mode (probably solved by optimization)\n";
                      continue; // Should result in mismatched load type, which will then segfault??
                      // If we chain these, do we need to do multiple rounds of queueing? In fact worse, if these butt up against each other, I may have to loop every time
                    }
                    queued_storeinsts.push_back(store_inst);
                  }
                }
              }
            }
          }
        }

        // Fix queued LoadInsts
        // TODO - defensive copies? Are these even necessary?
        for (auto* load_inst: queued_loadinsts) {
          auto* gep_inst = dyn_cast<GetElementPtrInst>(load_inst->getPointerOperand());
          auto* prev_load_inst = dyn_cast<LoadInst>(gep_inst->getPointerOperand());


          // errs() << *load_inst << "\n";
        }

        // Fix queued StoreInsts
        // TODO - defensive copies Are these even necessary?
        for (auto* store_inst: queued_storeinsts) {
          auto* gep_inst = dyn_cast<GetElementPtrInst>(store_inst->getPointerOperand());
          auto* prev_load_inst = dyn_cast<LoadInst>(gep_inst->getPointerOperand());

          // TODO - check above, if failures, abort. This ties in with earlier question about restarting the loop each time.

          int32_t field_index;
          if (auto* maybe_index = dyn_cast<ConstantInt>(gep_inst->getOperand(gep_inst->getNumIndices()))) {
            field_index = maybe_index->getZExtValue();
          } else {
            errs() << "UNDEFINED BEHAVIOR 1!!\n"; // TODO - make this an actual error
            continue;
          }
          auto struct_name = getStrippedStructName(gep_inst->getPointerOperandType());

          // need new: gep, then load, then call set()
          auto* constantint = ConstantInt::get(int32arg_t, 0);
          std::vector<Value*> constantarray = {constantint, constantint};
          auto arrayref = ArrayRef<Value*>(constantarray);
          auto* new_gep_inst = GetElementPtrInst::CreateInBounds(prev_load_inst->getPointerOperand(), arrayref);
          auto* new_load_inst = new LoadInst(int32arg_t, new_gep_inst);


          std::string ffi_func_name = "set_field_" + std::to_string(field_index) + "_in_" + struct_name + "_ffi";
          // TODO - running into problems where I can't see header definitions in the Module unless they are used
          auto ffi_func = M.getOrInsertFunction(ffi_func_name, Type::getVoidTy(ctx), int32arg_t, store_inst->getValueOperand()->getType());
          // if (!ffi_func) {
          //   errs() << "UNDEFINED BEHAVIOR 2!!\n"; // TODO - make this an actual error
          //   continue;
          // }

          std::vector<Value*> args_vector = {new_load_inst, store_inst->getValueOperand()};
          auto args_arrayref = ArrayRef<Value*>(args_vector);
          auto* ffi_call_inst = CallInst::Create(ffi_func, args_arrayref);

          ReplaceInstWithInst(store_inst, ffi_call_inst);
          new_load_inst->insertBefore(ffi_call_inst);
          new_gep_inst->insertBefore(new_load_inst);

          gep_inst->eraseFromParent();
          prev_load_inst->eraseFromParent();

        }

        // Now that the old function is dead, delete it.
        oldFun->eraseFromParent();

        // auto* getelemptr_inst = getFirstGetElemPtrForStruct(f, structname);
        // if (!getelemptr_inst) return false;

        // auto* load_or_store_inst = getelemptr_inst->getNextNonDebugInstruction();
        // if (auto* load_inst = dyn_cast_or_null<LoadInst>(getelemptr_inst)) {
        //   correctGetInsts(load_inst);
        //   // return transformation(f, structname);
        //   // TODO - running the whole process for each instance of a get or set is expensive. But, it is more safe.
        //   //        costs are incurred at compile time and not run time.
        // }
        // else if (auto* store_inst = dyn_cast_or_null<StoreInst>(getelemptr_inst)) {
        //   correctSetInsts(store_inst);
        //   // return transformation(f, structname);
        // }
        errs() << "At least we could make the transformation?\n";
        unsafeFuncProbing(*newFun);
        verifyFunction(*newFun, &errs());
        errs() << "\nMade it through safely (enough)\n";
        isChanged = true;
      }
      // bool passesCheck = verifyModule(M, &(errs()));
      return isChanged;
    }
  };
}


char FakePtrPass::ID = 0;
static RegisterPass<FakePtrPass> X("fakeptr", "Fake Ptr Pass");

static void registerFakePtrPass(const PassManagerBuilder &Builder, legacy::PassManagerBase &PM) {
  PM.add(new FakePtrPass());
}

static RegisterStandardPasses clangtoolLoader_Ox(PassManagerBuilder::EP_ModuleOptimizerEarly, registerFakePtrPass);
static RegisterStandardPasses clangtoolLoader_O0(PassManagerBuilder::EP_EnabledOnOptLevel0, registerFakePtrPass);
// static RegisterStandardPasses registerMyPass(PassManagerBuilder::EP_EarlyAsPossible, registerFakePtrPass);

// Automatically enable the pass.
// http://adriansampson.net/blog/clangpass.html
// static void registerFakePtrPass(const PassManagerBuilder &,
//                          legacy::PassManagerBase &PM) {
//   PM.add(new FakePtrPass());
// }
// static RegisterStandardPasses
//   RegisterMyPass(PassManagerBuilder::EP_EarlyAsPossible,
//                  registerFakePtrPass);
