<template>
	<canvas class="size-full" id="about_scene" />
</template>

<script setup lang="ts">
import * as THREE from 'three'
import { GLTFLoader, type GLTF } from 'three/examples/jsm/Addons.js'
import { onMounted, onScopeDispose, useTemplateRef } from 'vue'

function loadGLTF(url: string): Promise<GLTF> {
	return new Promise((res, rej) => {
		const loader = new GLTFLoader()
		loader.load(
			url,
			(data) => {
				res(data)
			},
			undefined,
			rej,
		)
	})
}

function createTip(position: THREE.Vector3, color: THREE.ColorRepresentation = 0x00ff00) {
	const tipGeometry = new THREE.SphereGeometry(2)
	const tipMaterial = new THREE.MeshBasicMaterial({ color })
	const tipMesh = new THREE.Mesh(tipGeometry, tipMaterial)
	tipMesh.position.copy(position)
	return tipMesh
}

function createWaterMaterial(): THREE.ShaderMaterial {
	return new THREE.ShaderMaterial({
		uniforms: {
			time: { value: 0 },
			seed: { value: Math.random() + 10 },
			color: { value: new THREE.Color(0.3, 0.3, 1.0) },
		},
		transparent: true,
		vertexShader: `#define WATER_VERT
varying vec2 vUv;
void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}`,
		fragmentShader: `#define WATER_FRAG
uniform float time;
uniform float seed;
uniform vec3 color;
varying vec2 vUv;

vec2 randomGradient(vec2 p) {
    float n = sin(dot(p, vec2(127.1, 311.7)));
    float angle = fract(n * 43758.5453123) * 6.28318530718 * seed;
    return vec2(cos(angle), sin(angle));
}

float perlinNoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);

    vec2 u = f * f * (3.0 - 2.0 * f);

    vec2 g1 = randomGradient(i);
    vec2 g2 = randomGradient(i + vec2(1.0, 0.0));
    vec2 g3 = randomGradient(i + vec2(0.0, 1.0));
    vec2 g4 = randomGradient(i + vec2(1.0, 1.0));

    vec2 d1 = f;
    vec2 d2 = f - vec2(1.0, 0.0);
    vec2 d3 = f - vec2(0.0, 1.0);
    vec2 d4 = f - vec2(1.0, 1.0);

    float v1 = dot(g1, d1);
    float v2 = dot(g2, d2);
    float v3 = dot(g3, d3);
    float v4 = dot(g4, d4);

    return mix(mix(v1, v2, u.x), mix(v3, v4, u.x), u.y);
}

void main() {
    float height = 0.0;
    height += perlinNoise(vec2(vUv.x * 10.0, time * 0.8)) * 0.2;
    height += perlinNoise(vec2(vUv.x * 5.0, time * 0.4)) * 0.3;
    height += perlinNoise(vec2(vUv.x * 2.5, time * 0.2)) * 0.3;
    height += perlinNoise(vec2(vUv.x * 2.0, time * 0.2)) * 0.2;
    height = clamp(height, -1.0, 1.0);
    height = height * 0.5 + 0.8;

    float thickness = 0.01;
    if(vUv.y < height - thickness) {
        gl_FragColor = vec4(color, 0.8);
    } else if(vUv.y > height + thickness) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 0.0);
    } else {
        gl_FragColor = vec4(color, 1.0);
    }
}`,
	})
}

function createWater(material: THREE.ShaderMaterial, x: number, y: number, z: number) {
	const waterGeometry = new THREE.PlaneGeometry(70, 16)
	const waterMesh = new THREE.Mesh(waterGeometry, material)
	waterMesh.position.add(new THREE.Vector3(x, y, z))
	return waterMesh
}

function main() {
	const canvas = document.querySelector('#about_scene')
	if (!canvas) return console.error('No canvas')

	const canvasSize = new THREE.Vector2(
		canvas.getBoundingClientRect().width,
		canvas.getBoundingClientRect().height,
	)

	const renderer = new THREE.WebGLRenderer({
		antialias: true,
		alpha: true,
		canvas,
	})
	renderer.setPixelRatio(devicePixelRatio)
	renderer.setSize(canvasSize.x, canvasSize.y)

	const deltaClock = new THREE.Clock()
	const elapseClock = new THREE.Clock()
	deltaClock.start()
	elapseClock.start()

	const scene = new THREE.Scene()

	const camera = new THREE.PerspectiveCamera(30, canvasSize.x / canvasSize.y, 1, 3000)
	camera.position.set(-10, 5, 30)
	camera.lookAt(0, 0, 0)

	const ambientLight = new THREE.AmbientLight(0xffffff)
	scene.add(ambientLight)

	const dirLight = new THREE.DirectionalLight(0xffffff, 5.0)
	dirLight.position.set(-30, 30, 28)
	scene.add(dirLight)

	scene.add(createTip(dirLight.position, 0xffff00))
	scene.add(createTip(camera.position))

	const waterMaterial = createWaterMaterial()
	scene.add(createWater(waterMaterial, 0, -6.5, 4))
	scene.add(createWater(waterMaterial, -1, -8, -4))

	const accentColor =
		getComputedStyle(document.documentElement).getPropertyValue('--color-brand').trim() || '#4444ff'

	waterMaterial.uniforms.color.value = new THREE.Color(accentColor).multiplyScalar(0.8)

	async function load() {
		const axlGLTF = await loadGLTF('/models/axolotl.gltf')

		const axlModel = axlGLTF.scene
		axlModel.scale.multiplyScalar(5)
		axlModel.rotateY(Math.PI / 2)
		axlModel.position.add(new THREE.Vector3(0, -2.5, 0))
		scene.add(axlModel)

		const mixer = new THREE.AnimationMixer(axlModel)
		const axlSwimAnim = axlGLTF.animations.filter((a) => a.name === 'swim')[0]
		if (!axlSwimAnim) return console.error('Missing animation swim')
		mixer.clipAction(axlSwimAnim).play()

		const axlLabelGLTF = await loadGLTF('/models/axl_label.glb')
		const axlLabel = axlLabelGLTF.scene
		axlLabel.scale.multiplyScalar(8)
		axlLabel.rotateY(-Math.PI / 2)
		axlLabel.position.set(0, 5.2, 0)

		// scene.add(axlLabel)

		const originAxlModelPosition = axlModel.position.clone()
		return function (deltaTime: number, elapsedTime: number) {
			axlModel.position.set(
				originAxlModelPosition.x,
				originAxlModelPosition.y + Math.sin(elapsedTime),
				originAxlModelPosition.z,
			)
			axlModel.rotation.y = Math.sin(elapsedTime * 0.3) * 0.2 + Math.PI / 2
			mixer.update(deltaTime)
			// console.log(timer.getDelta() * 1000)
		}
	}
	let updateGLTF = (deltaTime: number, elapsedTime: number) => {}
	load().then((updateFn) => {
		if (updateFn) updateGLTF = updateFn
	})

	function animate(time: number) {
		requestAnimationFrame(animate)

		const deltaTime = deltaClock.getDelta()
		const elapsedTime = elapseClock.getElapsedTime()

		updateGLTF(deltaTime, elapsedTime)
		waterMaterial.uniforms.time.value = elapsedTime

		renderer.render(scene, camera)
	}
	animate(Date.now())

	const originCameraPosition = camera.position.clone()
	function onMouseMove(event: MouseEvent) {
		const mouseXOffsetRatio = ((event.clientX - innerWidth / 2) / innerWidth) * 2
		const mouseYOffsetRatio = ((event.clientY - innerHeight / 2) / innerHeight) * 2
		const newPosition = new THREE.Vector3(
			originCameraPosition.x + mouseXOffsetRatio,
			originCameraPosition.y + mouseYOffsetRatio * 0.5,
			originCameraPosition.z,
		)
		camera.position.copy(newPosition)
	}

	var isUpdating = true
	function updateSize() {
		if (!isUpdating) return
		if (!canvas) return
		const rect = canvas.getBoundingClientRect()
		const w = rect.width
		const h = rect.height
		if (w > 0 && h > 0) {
			renderer.setSize(w, h)
			camera.aspect = w / h
			camera.updateProjectionMatrix()
		}
	}

	const resizeObserver = new ResizeObserver(updateSize)
	resizeObserver.observe(canvas)

	addEventListener('mousemove', onMouseMove)
	onScopeDispose(() => {
		isUpdating = false
		removeEventListener('mousemove', onMouseMove)
		resizeObserver.disconnect()
		deltaClock.stop()
		elapseClock.stop()
		renderer.dispose()
	})
}
// main();
onMounted(main)
</script>
