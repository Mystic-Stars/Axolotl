import { AbstractPopupNotificationManager, type PopupNotification } from '@modrinth/ui'
import { type Ref, ref } from 'vue'

export class AppPopupNotificationManager extends AbstractPopupNotificationManager {
	private static readonly STORAGE_KEY = 'axolotl:dismissed-popup-notifications-v2'
	private static readonly LEGACY_STORAGE_KEY = 'axolotl:dismissed-popup-notifications'
	private readonly state: Ref<PopupNotification[]>
	private readonly dismissedKeys: Set<string>

	public constructor() {
		super()
		this.state = ref<PopupNotification[]>([])
		this.dismissedKeys = this.loadDismissedKeys()
	}

	public getNotifications(): PopupNotification[] {
		return this.state.value
	}

	protected addNotificationToStorage(notification: PopupNotification): void {
		if (notification.type !== 'download' && this.dismissedKeys.has(this.key(notification))) return
		this.state.value.unshift(notification)
	}

	protected removeNotificationFromStorage(id: string | number): void {
		const index = this.state.value.findIndex((n) => n.id === id)
		if (index > -1) this.state.value.splice(index, 1)
	}

	protected clearAllNotificationsFromStorage(): void {
		this.state.value.splice(0)
	}

	public override removeNotification = (id: string | number): void => {
		const notification = this.state.value.find((item) => item.id === id)
		super.removeNotification(id)
		if (notification && notification.type !== 'download') {
			this.dismissedKeys.add(this.key(notification))
			this.saveDismissedKeys()
		}
	}

	public override clearAllNotifications = (): void => {
		for (const notification of this.state.value) {
			if (notification.type !== 'download') this.dismissedKeys.add(this.key(notification))
		}
		super.clearAllNotifications()
		this.saveDismissedKeys()
	}

	private key(notification: PopupNotification): string {
		return JSON.stringify([
			notification.title,
			notification.text ?? '',
			notification.type ?? '',
			notification.toast?.type ?? '',
			notification.toast?.actorName ?? '',
			notification.toast?.entityName ?? '',
		])
	}

	private loadDismissedKeys(): Set<string> {
		try {
			const value = JSON.parse(
				localStorage.getItem(AppPopupNotificationManager.STORAGE_KEY) ?? '[]',
			)
			const keys = new Set(
				Array.isArray(value) ? value.filter((key): key is string => typeof key === 'string') : [],
			)
			const legacy = JSON.parse(
				localStorage.getItem(AppPopupNotificationManager.LEGACY_STORAGE_KEY) ?? '{}',
			)
			if (Array.isArray(legacy.keys)) {
				for (const key of legacy.keys) {
					if (typeof key !== 'string') continue
					try {
						const parsed = JSON.parse(key)
						if (Array.isArray(parsed) && parsed.length >= 3) {
							keys.add(JSON.stringify([parsed[0], parsed[1] ?? '', parsed[2] ?? '', '', '', '']))
						}
					} catch {
						// Ignore malformed legacy keys.
					}
				}
			}
			return keys
		} catch {
			return new Set()
		}
	}

	private saveDismissedKeys(): void {
		try {
			localStorage.setItem(
				AppPopupNotificationManager.STORAGE_KEY,
				JSON.stringify([...this.dismissedKeys].slice(-100)),
			)
		} catch {
			// Popup notifications remain usable when storage is unavailable.
		}
	}
}
