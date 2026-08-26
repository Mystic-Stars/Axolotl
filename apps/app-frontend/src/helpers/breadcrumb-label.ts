export function resolveBreadcrumbLabel<Message>(
	name: string,
	getDynamicName: (key: string) => string,
	staticLabels: Readonly<Record<string, Message>>,
	formatMessage: (message: Message) => string,
): string {
	if (name.startsWith('?')) return getDynamicName(name.slice(1))
	const label = staticLabels[name]
	return label === undefined ? name : formatMessage(label)
}
