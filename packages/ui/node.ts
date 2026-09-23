// Node test entrypoint. Keep this independent from the browser component tree
// and from extensionless TypeScript imports that Node cannot resolve directly.
export function defineMessage(descriptor) {
	return descriptor
}

export function defineMessages(descriptors) {
	return descriptors
}

const messageProxy = new Proxy(
	{},
	{
		get: (_target, key) => ({
			id: `ui.${String(key)}`,
			defaultMessage: String(key),
		}),
	},
)

export const commonMessages = messageProxy
export const formFieldLabels = messageProxy
export const formFieldPlaceholders = messageProxy
