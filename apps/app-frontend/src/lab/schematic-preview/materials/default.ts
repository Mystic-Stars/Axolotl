import {
	CanvasTexture,
	ShaderLib,
	ShaderMaterial,
	UniformsUtils,
	Vector2,
	type IUniform,
} from 'three'

export class DefaultMaterial extends ShaderMaterial {
	constructor(
		translucent: boolean,
		uniforms: {
			clipY: IUniform<Vector2>
			[uniform: string]: IUniform<any>
		},
		map?: CanvasTexture,
	) {
		var mergedUniforms = UniformsUtils.merge([ShaderLib.lambert.uniforms, uniforms])
		if (map) mergedUniforms.map.value = map
		super({
			uniforms: mergedUniforms,
			lights: true,
			fog: true,
			vertexColors: true,
			alphaTest: translucent ? 0 : 0.08,
			transparent: translucent,
			opacity: translucent ? 0.68 : 1,
			depthWrite: !translucent,

			vertexShader: `#define LAMBERT
uniform vec2 clipY;
attribute vec3 blockPosition;
varying vec3 vViewPosition;
varying vec2 vUv;
varying vec2 vMapUv;
varying float vShouldDiscard;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
	vMapUv = uv;

	vShouldDiscard = 0.0;
	if(blockPosition.y < clipY.x || blockPosition.y > clipY.y) {
		vShouldDiscard = 1.0;
	}
}
`,

			fragmentShader: `#define LAMBERT
#define USE_MAP
uniform vec3 diffuse;
uniform vec3 emissive;
uniform float opacity;
varying vec2 vUv;
varying float vShouldDiscard;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <cube_uv_reflection_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_lambert_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	if(vShouldDiscard >= 1.0) {
		discard;
	}
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_lambert_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + totalEmissiveRadiance;
	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}
`,
		})
	}

	setClipY(value: Vector2) {
		this.uniforms.clipY.value = value
	}
}
