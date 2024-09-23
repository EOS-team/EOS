// SPDX-License-Identifier: MIT
/*
 * Copyright © 2019 Intel Corporation
 */

#include "gem/i915_gem_region.h"
#include "intel_memory_region.h"

#include "mock_region.h"

static const struct drm_i915_gem_object_ops mock_region_obj_ops = {
	.name = "mock-region",
	.get_pages = i915_gem_object_get_pages_buddy,
	.put_pages = i915_gem_object_put_pages_buddy,
	.release = i915_gem_object_release_memory_region,
};

static int mock_object_init(struct intel_memory_region *mem,
			    struct drm_i915_gem_object *obj,
			    resource_size_t size,
			    unsigned int flags)
{
	static struct lock_class_key lock_class;
	struct drm_i915_private *i915 = mem->i915;

	if (size > mem->mm.size)
		return -E2BIG;

	drm_gem_private_object_init(&i915->drm, &obj->base, size);
	i915_gem_object_init(obj, &mock_region_obj_ops, &lock_class, flags);

	obj->read_domains = I915_GEM_DOMAIN_CPU | I915_GEM_DOMAIN_GTT;

	i915_gem_object_set_cache_coherency(obj, I915_CACHE_NONE);

	i915_gem_object_init_memory_region(obj, mem);

	return 0;
}

static const struct intel_memory_region_ops mock_region_ops = {
	.init = intel_memory_region_init_buddy,
	.release = intel_memory_region_release_buddy,
	.init_object = mock_object_init,
};

struct intel_memory_region *
mock_region_create(struct drm_i915_private *i915,
		   resource_size_t start,
		   resource_size_t size,
		   resource_size_t min_page_size,
		   resource_size_t io_start)
{
	return intel_memory_region_create(i915, start, size, min_page_size,
					  io_start, &mock_region_ops);
}
