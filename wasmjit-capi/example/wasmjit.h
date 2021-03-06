#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef uint8_t u8x6[4];

typedef struct {
  uint8_t *data;
  uint32_t len;
} wasmjit_slice_t;

typedef struct {
  uint8_t *data;
  uint32_t len;
} wasmjit_bytes_t;

typedef struct {
  uint8_t _unused[0];
} wasmjit_chain_context_t;

typedef uint8_t h256_t[32];

typedef uint8_t address_t[20];

typedef uint32_t wasmjit_result_kind;

typedef struct {
  wasmjit_result_kind kind;
  wasmjit_bytes_t msg;
} wasmjit_result_t;

typedef struct {
  uint8_t _unused[0];
} wasmjit_module_t;

typedef struct {
  uint8_t _unused[0];
} wasmjit_instance_t;

typedef struct {
  uint8_t _unused[0];
} wasmjit_resolver_t;

typedef struct {
  uint8_t _unused[0];
} wasmjit_vmctx_t;

void abi_test(uint32_t a1,
              uint32_t a2,
              uint64_t a3,
              uint64_t a4,
              uint64_t a5,
              uint64_t a6,
              const u8x6 *a7);

wasmjit_slice_t wasmjit_bytes_as_slice(wasmjit_bytes_t bytes);

void wasmjit_bytes_destroy(wasmjit_bytes_t bytes);

wasmjit_bytes_t wasmjit_bytes_new(uint32_t len);

wasmjit_chain_context_t *wasmjit_chain_context_create(uint32_t height,
                                                      h256_t *blockhash,
                                                      uint64_t timestamp,
                                                      h256_t *txhash,
                                                      wasmjit_slice_t callers_raw,
                                                      wasmjit_slice_t witness_raw,
                                                      wasmjit_slice_t input_raw,
                                                      uint64_t exec_step,
                                                      uint64_t gas_factor,
                                                      uint64_t gas_left,
                                                      uint64_t depth_left,
                                                      uint64_t service_index);

void wasmjit_chain_context_destroy(wasmjit_chain_context_t *ctx);

uint64_t wasmjit_chain_context_get_exec_step(wasmjit_chain_context_t *ctx);

uint64_t wasmjit_chain_context_get_gas(wasmjit_chain_context_t *ctx);

void wasmjit_chain_context_pop_caller(wasmjit_chain_context_t *ctx, address_t *result);

void wasmjit_chain_context_push_caller(wasmjit_chain_context_t *ctx, const address_t *caller);

void wasmjit_chain_context_set_exec_step(wasmjit_chain_context_t *ctx, uint64_t exec_step);

void wasmjit_chain_context_set_gas(wasmjit_chain_context_t *ctx, uint64_t gas);

void wasmjit_chain_context_set_output(wasmjit_chain_context_t *ctx, wasmjit_bytes_t bytes);

wasmjit_bytes_t wasmjit_chain_context_take_output(wasmjit_chain_context_t *ctx);

wasmjit_result_t wasmjit_compile(wasmjit_module_t **compiled, wasmjit_slice_t wasm);

void wasmjit_instance_destroy(wasmjit_instance_t *instance);

wasmjit_result_t wasmjit_instance_invoke(wasmjit_instance_t *instance,
                                         wasmjit_chain_context_t *ctx);

wasmjit_result_t wasmjit_instantiate(wasmjit_instance_t **instance,
                                     wasmjit_resolver_t *resolver,
                                     wasmjit_slice_t wasm);

void wasmjit_module_destroy(wasmjit_module_t *module);

wasmjit_result_t wasmjit_module_instantiate(const wasmjit_module_t *module,
                                            wasmjit_resolver_t *resolver,
                                            wasmjit_instance_t **instance);

void wasmjit_resolver_destroy(wasmjit_resolver_t *resolver);

wasmjit_resolver_t *wasmjit_simple_resolver_create(void);

wasmjit_bytes_t wasmjit_test_read_wasm_file(wasmjit_slice_t name);

wasmjit_result_t wasmjit_validate(wasmjit_slice_t wasm);

/**
 * Implementation of wasmjit_vmctx_chainctx
 */
wasmjit_chain_context_t *wasmjit_vmctx_chainctx(wasmjit_vmctx_t *vmctx);

wasmjit_result_t wasmjit_vmctx_memory(wasmjit_vmctx_t *ctx, wasmjit_slice_t *result);
