import { defineAdminHandler } from '../../utils/handler'
import { getAdminApi } from '../../utils/service'
import { getQueryRecord, parseRange } from '../../utils/validation'

export default defineAdminHandler(async (event) =>
    getAdminApi(event).overview(parseRange(getQueryRecord(event).range)),
)
