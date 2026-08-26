export namespace Mclogs {
	export namespace Logs {
		export namespace v1 {
			export type CreateResponse = {
				success: boolean
				id: string
				source: string | null
				created: number
				expires: number
				size: number
				lines: number
				errors: number
				url: string
				raw: string
				token: string
			}
		}
	}
}
