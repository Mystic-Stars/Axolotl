export class AdminApiError extends Error {
    constructor(
        public readonly statusCode: number,
        public readonly code: string,
        message: string,
    ) {
        super(message)
        this.name = 'AdminApiError'
    }
}

export const unauthorized = () => new AdminApiError(401, 'unauthenticated', '需要登录')
export const forbidden = () => new AdminApiError(403, 'forbidden', '当前身份无权访问')
export const unavailable = () => new AdminApiError(503, 'service_unavailable', '遥测数据暂不可用')
