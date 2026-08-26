const apiToken = process.env.CF_API_TOKEN
const zoneId = process.env.CF_ZONE_ID

if (!apiToken || !zoneId) {
	throw new Error('CF_API_TOKEN and CF_ZONE_ID are required')
}

const apiBase = `https://api.cloudflare.com/client/v4/zones/${zoneId}/rulesets`
const ruleDescription = 'Axolotl telemetry batch ingress limit'
const telemetryRule = {
	action: 'block',
	description: ruleDescription,
	enabled: true,
	expression:
		'http.host eq "telemetry.axlmc.org" and http.request.method eq "POST" and http.request.uri.path eq "/v1/batch"',
	action_parameters: {
		ratelimit: {
			characteristics: ['ip.src'],
			period: 60,
			requests_per_period: 2,
			mitigation_timeout: 60,
			requests_to_origin: false,
		},
	},
}

async function request(path = '', options = {}) {
	const response = await fetch(`${apiBase}${path}`, {
		...options,
		headers: {
			Authorization: `Bearer ${apiToken}`,
			'Content-Type': 'application/json',
			...(options.headers ?? {}),
		},
	})
	const payload = await response.json()
	if (!response.ok || !payload.success) {
		throw new Error(`Cloudflare API request failed: ${JSON.stringify(payload.errors ?? payload)}`)
	}
	return payload.result
}

function editableRule(rule) {
	const { action, action_parameters, description, enabled, expression, id, logging } = rule
	return { action, action_parameters, description, enabled, expression, id, logging }
}

const rulesets = await request('?phase=http_ratelimit')
const rateLimitRuleset = rulesets.find((ruleset) => ruleset.phase === 'http_ratelimit')

if (!rateLimitRuleset) {
	await request('', {
		method: 'POST',
		body: JSON.stringify({
			kind: 'zone',
			name: 'Axolotl rate limits',
			phase: 'http_ratelimit',
			rules: [telemetryRule],
		}),
	})
	console.log('Created the Axolotl telemetry batch rate-limit rule')
} else {
	const existing = await request(`/${rateLimitRuleset.id}`)
	const rules = existing.rules.map(editableRule)
	const index = rules.findIndex((rule) => rule.description === ruleDescription)
	if (index >= 0) {
		rules[index] = { ...telemetryRule, id: rules[index].id }
	} else {
		rules.push(telemetryRule)
	}
	await request(`/${rateLimitRuleset.id}`, {
		method: 'PUT',
		body: JSON.stringify({
			name: existing.name,
			kind: existing.kind,
			phase: existing.phase,
			rules,
		}),
	})
	console.log('Updated the Axolotl telemetry batch rate-limit rule')
}
