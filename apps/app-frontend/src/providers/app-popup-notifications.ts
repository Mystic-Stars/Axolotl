import { AbstractPopupNotificationManager, type PopupNotification } from '@modrinth/ui'
import { type Ref, ref } from 'vue'

import { loadNotificationDismissals } from './notification-dismissals.ts'

export class AppPopupNotificationManager extends AbstractPopupNotificationManager {
	private static readonly STORAGE_KEY = 'axolotl:dismissed-popup-notifications-v2'
	private static readonly LEGACY_STORAGE_KEY = 'axolotl:dismissed-popup-notifications'
	private readonly state: Ref<PopupNotification[]>
	private readonly dismissedKeys: Set<string>
	private dismissedBefore: number | null

	public constructor() {
		super()
		this.state = ref<PopupNotification[]>([])
		const dismissed = loadNotificationDismissals(
			[AppPopupNotificationManager.STORAGE_KEY, AppPopupNotificationManager.LEGACY_STORAGE_KEY],
			6,
		)
		this.dismissedKeys = dismissed.keys
		this.dismissedBefore = dismissed.clearedAt
	}

	public getNotifications(): PopupNotification[] {
		return this.state.value
	}

	protected addNotificationToStorage(notification: PopupNotification): void {
		if (
			this.dismissedKeys.has(this.key(notification)) ||
			(this.dismissedBefore !== null && (notification.createdAt ?? 0) <= this.dismissedBefore)
		)
			return
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
		if (notification) {
			this.dismissedKeys.add(this.key(notification))
			this.saveDismissedKeys()
		}
	}

	public override clearAllNotifications = (): void => {
		this.dismissedBefore = Date.now()
		for (const notification of this.state.value) {
			this.dismissedKeys.add(this.key(notification))
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
			...(notification.progressItems
				? [
						notification.progressItems
							.map((item) => item.id)
							.sort()
							.join('|'),
					]
				: []),
		])
	}

	private saveDismissedKeys(): void {
		try {
			localStorage.setItem(
				AppPopupNotificationManager.STORAGE_KEY,
				JSON.stringify({
					clearedAt: this.dismissedBefore,
					keys: [...this.dismissedKeys].slice(-100),
				}),
			)
		} catch {
			// Popup notifications remain usable when storage is unavailable.
		}
	}
}
