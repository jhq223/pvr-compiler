/* Host adapter for the local PVR GLES2 glCompileShader path. No EGL, GPU service
 * or VitaSDK entry points are linked. Compiler algorithms stay in vendor/. */
#include <stdlib.h>
#include <string.h>
#include "glsl2uf.h"
#include "use.h"
#include "constants.h"
#include "esbinshader.h"

static void *binary_malloc(IMG_UINT32 size) { return malloc(size); }

void *pvr_compiler_create(void)
{
    GLSLInitCompilerContext *init = calloc(1, sizeof(*init));
    GLSLRequestedPrecisions *p;
    GLSLCompilerResources *r;
    if (!init) return NULL;
    p = &init->sRequestedPrecisions;
    r = &init->sCompilerResources;

    /* shader.c SetPrecision(..., 0), matching the default apphint. */
    p->eDefaultUserVertFloat = p->eDefaultUserVertInt = GLSLPRECQ_HIGH;
    p->eDefaultUserVertSampler = p->eDefaultUserFragSampler = GLSLPRECQ_LOW;
    p->eDefaultUserFragInt = GLSLPRECQ_MEDIUM;
    p->eVertBooleanPrecision = p->eFragBooleanPrecision = GLSLPRECQ_HIGH;
    p->eBIStateInt = p->eGLPosition = p->eDepthRange = GLSLPRECQ_HIGH;
    p->eBIFragFloat = p->eGLPointSize = p->eGLPointCoord = GLSLPRECQ_MEDIUM;
    r->iGLMaxVertexAttribs = GLES2_MAX_VERTEX_ATTRIBS;
    r->iGLMaxVertexUniformVectors = GLES2_MAX_VERTEX_UNIFORM_VECTORS;
    r->iGLMaxVaryingVectors = GLES2_MAX_VARYING_VECTORS;
    r->iGLMaxVertexTextureImageUnits = GLES2_MAX_VERTEX_TEXTURE_UNITS;
    r->iGLMaxCombinedTextureImageUnits = r->iGLMaxTextureImageUnits = GLES2_MAX_TEXTURE_UNITS;
    r->iGLMaxFragmentUniformVectors = GLES2_MAX_FRAGMENT_UNIFORM_VECTORS;
    r->iGLMaxDrawBuffers = GLES2_MAX_DRAW_BUFFERS;
    init->sInlineFuncRules.bInlineCalledOnceFunc = IMG_TRUE;
    init->sInlineFuncRules.bInlineSamplerParamFunc = IMG_TRUE;
    init->sInlineFuncRules.uNumICInstrsBodyLessThan = 10;
    init->sInlineFuncRules.uNumParamComponentsGreaterThan = 32;
    init->sUnrollLoopRules.bEnableUnroll = IMG_TRUE;
    init->sUnrollLoopRules.bUnrollRelativeAddressingOnly = IMG_TRUE;
    init->sUnrollLoopRules.uMaxNumIterations = 50;
    if (!GLSLInitCompiler(init)) {
        free(init);
        return NULL;
    }
    return init;
}

int pvr_compiler_compile(void *compiler, const char *source, int fragment, char **log, unsigned int *binary_bytes, void **binary)
{
    GLSLInitCompilerContext *init = compiler;
    GLSLCompileProgramContext context = {0};
    GLSLCompileUniflexProgramContext compile = {0};
    UNIFLEX_PROGRAM_PARAMETERS params = {0};
    GLSLUniFlexHWCodeInfo hw = {0};
    GLSLCompiledUniflexProgram *program;
    int success;
    *binary_bytes = 0;

    /* SGX543 has 384 global temporary registers; reserve a 2x2 pixel block.
     * Reserved secondary registers follow GLES2 usegles2.h / shader.c. */
    params.uNumAvailableTemporaries = EURASIA_USE_GLOBAL_TEMP_REG_LIMIT >> 2;
    params.uConstantBase = 0;
    params.uIndexableTempBase = 1;
    params.uScratchBase = fragment ? 5 : 6;
    params.uInRegisterConstantOffset = fragment ? 9 : 10;
    params.uInRegisterConstantLimit = (fragment ? 128 : 512) - params.uInRegisterConstantOffset;
    params.ePredicationLevel = UF_PREDLVL_AUTO;
    if (fragment) {
        params.uPackDestType = USEASM_REGTYPE_PRIMATTR;
        params.uPackPrecision = 5;
    }
    hw.psUFParams = &params;
    context.psInitCompilerContext = init;
    context.ppszSourceCodeStrings = (IMG_CHAR **)&source;
    context.uNumSourceCodeStrings = 1;
    context.eProgramType = fragment ? GLSLPT_FRAGMENT : GLSLPT_VERTEX;
    context.bCompleteProgram = IMG_TRUE;
    context.eEnabledWarnings = 0x7fffffff;
    compile.eOutputCodeType = GLSLPF_UNIFLEX_OUTPUT;
    compile.psUniflexHWCodeInfo = &hw;
    compile.psCompileProgramContext = &context;
    program = GLSLCompileToUniflex(&compile);
    if (!program) {
        *log = strdup("PVR compiler returned no program");
        return -1;
    }
    success = program->bSuccessfullyCompiled ? 1 : 0;
    if (program->psUniFlexCode && program->psUniFlexCode->psUniPatchInput)
        *binary_bytes = sizeof(USP_PC_SHADER) + program->psUniFlexCode->psUniPatchInput->uSize;
    *log = strdup(program->sInfoLog.pszInfoLogString ? program->sInfoLog.pszInfoLogString : "");
    if (binary) {
        *binary = NULL;
        if (success && SGXBS_CreateBinaryShader(program, binary_malloc, free, binary, binary_bytes) != SGXBS_NO_ERROR) {
            /* The packer frees failed output itself. Never expose its pointer. */
            *binary = NULL;
            free(*log);
            *log = strdup("PVR binary shader serialization failed");
            success = -1;
        }
    }
    GLSLFreeCompiledUniflexProgram(init, program);
    return success;
}

void pvr_compiler_free_log(char *log) { free(log); }

void pvr_compiler_destroy(void *compiler)
{
    GLSLShutDownCompiler(compiler);
    free(compiler);
}
