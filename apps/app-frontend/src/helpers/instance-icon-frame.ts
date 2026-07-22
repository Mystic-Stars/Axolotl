const builtInInstanceIconHashes = new Set([
	'2bea5c08afa67675f49b70194990de0e20c0ef2e',
	'1db2c92a33ef1f6f490ec27070d49cb8353bad7c',
	'a55366513a44c3d77a4b4c1004728fb3b35943d8',
	'7af62e3150462a95959813d6f1cb5790d8964b8c',
	'289bb4e05e7b59c381586f5b9cb6f9ed8c8aaa9e',
	'304c474d04fd264a08a09579491dc6885148519c',
	'3a3537adb1f5f4986c51ce7291323b8d63cdacb6',
	'17079681f0b24fb14146130110e41fa31fdfc2fd',
	'92322c1a03ac054fa8dae65f58460c897941190c',
	'e0eb7788d1ba7efcedddc4bd36d0e6d0cd681332',
	'a8031d4bedb57a5bc2ae518e44758f6218e1cb0e',
	'1b18b5576b932db211df122c0b47eeeb1e9eaed3',
	'591c36b5b9f3317ccfc50182ca8c2a2090cde76a',
	'a8605b9c54e09a19d4b4f93e60f0e2b6a837aef4',
	'10a23de727a86fa8de557edb7792a68d379b8731',
	'77ccc6d5a44e1316de9c18d56367cbfd1b7e64b6',
	'e5a333971e4acbc2b1112f58d0f84f1f36ef7411',
	'10d2332465a667ab338a69ee59cc4c9c0531de8f',
	'a975108ca40a06718b504a411ddd7c4944113ac3',
	'4c680e8810b69ded4e94d372d150bd910f0cc592',
])

export function isBuiltInInstanceIcon(iconPath: string | null | undefined): boolean {
	if (!iconPath) return false
	const match = iconPath.match(/([a-f\d]{40})(?:\.[^/?#\\]+)?(?:[?#].*)?$/i)
	return match ? builtInInstanceIconHashes.has(match[1].toLowerCase()) : false
}
