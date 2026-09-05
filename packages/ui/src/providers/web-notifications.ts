import { createContext } from '.'

export interface WebNotification {
	id: string | number
	createdAt?: number
	title?: string
	text?: string
	type?: 'error' | 'warning' | 'success' | 'info'
	errorCode?: string
	count?: number
	autoCloseMs?: number | null // null means do not dismiss automatically
	timer?: NodeJS.Timeout
	/** Hidden from the toast stack but retained in notification history. */
	collapsed?: boolean
	supportData?: Record<string, unknown>
}

export type NotificationPanelLocation = 'left' | 'right'

export abstract class AbstractWebNotificationManager {
	protected readonly DEFAULT_AUTO_DISMISS_DELAY_MS = 30 * 1000
	private lastGeneratedNotificationId = 0

	abstract getNotifications(): WebNotification[]
	abstract getNotificationLocation(): NotificationPanelLocation
	abstract setNotificationLocation(location: NotificationPanelLocation): void

	protected abstract addNotificationToStorage(notification: WebNotification): void
	protected abstract removeNotificationFromStorage(id: string | number): void
	protected abstract removeNotificationFromStorageByIndex(index: number): void
	protected abstract clearAllNotificationsFromStorage(): void

	addNotification = (notification: Partial<WebNotification>): WebNotification => {
		const existingNotif = this.findExistingNotification(notification)

		if (existingNotif) {
			existingNotif.createdAt = Date.now()
			existingNotif.collapsed = false
			this.refreshNotificationTimer(existingNotif)
			existingNotif.count = (existingNotif.count || 0) + 1
			return existingNotif
		}

		const newNotification = this.createNotification(notification)
		this.setNotificationTimer(newNotification)
		this.addNotificationToStorage(newNotification)
		return newNotification
	}

	/**
	 * @deprecated You should use `addNotification` instead to provide a more human-readable error message to the user.
	 */
	handleError = (error: unknown): void => {
		this.addNotification({
			title: '发生错误',
			text:
				error instanceof Error
					? error.message
					: typeof error === 'string'
						? error
						: JSON.stringify(error),
			type: 'error',
		})
	}

	removeNotification = (id: string | number): WebNotification | undefined => {
		const notifications = this.getNotifications()
		const notification = notifications.find((n) => n.id === id)

		if (notification) {
			this.clearNotificationTimer(notification)
			this.removeNotificationFromStorage(id)
		}

		return notification
	}

	removeNotificationByIndex = (index: number): WebNotification | null => {
		const notifications = this.getNotifications()

		if (index >= 0 && index < notifications.length) {
			const notification = notifications[index]
			this.clearNotificationTimer(notification)
			this.removeNotificationFromStorageByIndex(index)

			return notification
		}

		return null
	}

	clearAllNotifications = (): void => {
		const notifications = this.getNotifications()
		notifications.forEach((notification) => {
			this.clearNotificationTimer(notification)
		})
		this.clearAllNotificationsFromStorage()
	}

	setNotificationTimer = (notification: WebNotification): void => {
		if (!notification) return

		this.clearNotificationTimer(notification)

		if (notification.autoCloseMs === null) return

		const delay = notification.autoCloseMs ?? this.DEFAULT_AUTO_DISMISS_DELAY_MS

		notification.timer = setTimeout(() => {
			this.collapseNotification(notification.id)
		}, delay)
	}

	collapseNotification = (id: string | number): void => {
		const notification = this.getNotifications().find((n) => n.id === id)
		if (notification) {
			this.clearNotificationTimer(notification)
			notification.collapsed = true
		}
	}

	expandNotification = (id: string | number): void => {
		const notification = this.getNotifications().find((n) => n.id === id)
		if (notification) {
			notification.collapsed = false
			this.setNotificationTimer(notification)
		}
	}

	stopNotificationTimer = (notification: WebNotification): void => {
		this.clearNotificationTimer(notification)
	}

	private refreshNotificationTimer(notification: WebNotification): void {
		this.setNotificationTimer(notification)
	}

	private clearNotificationTimer(notification: WebNotification): void {
		if (notification.timer) {
			clearTimeout(notification.timer)
			notification.timer = undefined
		}
	}

	private findExistingNotification(
		notification: Partial<WebNotification>,
	): WebNotification | undefined {
		return this.getNotifications().find(
			(existing) =>
				existing.text === notification.text &&
				existing.title === notification.title &&
				existing.type === notification.type,
		)
	}

	private createNotification(notification: Partial<WebNotification>): WebNotification {
		// Notifications can be created in the same millisecond. Keep generated
		// ids unique so dismissing one item never targets its siblings.
		const now = Date.now()
		const id = Math.max(now, this.lastGeneratedNotificationId + 1)
		this.lastGeneratedNotificationId = id

		return {
			...notification,
			id,
			createdAt: Date.now(),
			count: 1,
		} as WebNotification
	}
}

export const [injectNotificationManager, provideNotificationManager] =
	createContext<AbstractWebNotificationManager>('root', 'notificationManager')
