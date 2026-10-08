import { defineAdminHandler } from '../../utils/handler'
import { getAdminApi, requireSession } from '../../utils/service'

export default defineAdminHandler(async (event) => {
    const session = requireSession(event)
    getAdminApi(event)
    return { ...session, dataSource: session.mock ? 'fixture' : 'production' }
})
