#define _CRT_SECURE_NO_WARNINGS
void nk_rs_assert_fail(const char *expr, const char *file, int line);
#define NK_ASSERT(x) ((x) ? (void)0 : nk_rs_assert_fail(#x, __FILE__, __LINE__))
/* stb_truetype and stb_rect_pack keep their own assert macros. Both are guarded with #ifndef in nuklear.h. */
#define STBTT_assert(x) NK_ASSERT(x)
#define STBRP_ASSERT(x) NK_ASSERT(x)
#include "config.h"
#define NK_IMPLEMENTATION
#include "../nuklear/nuklear.h"
