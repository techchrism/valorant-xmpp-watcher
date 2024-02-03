import {promises as fs} from 'node:fs'
import {Logger} from "winston";
import fetch from "node-fetch";
import {Cookie} from 'set-cookie-parser'
import {createCookieString, mergeCookies, parseCookieString, parseSetCookieString} from './util/cookies'

// Modified from https://github.com/techchrism/valorant-api/blob/trunk/src/credentialManager/LocalCredentialManager.ts
// Subtracts this amount from expiration to avoid requesting resources with an about-to-expire cred
const expirationDiff = 60 * 1000

export class CredentialManager {
    private _entitlement: string | null = null
    private _token: string | null = null
    private _expiration: number = 0
    private _cookies: Cookie[] | null = null
    private readonly _logger: Logger

    constructor(logger: Logger) {
        this._logger = logger
    }

    async getToken(): Promise<string> {
        if(Date.now() > this._expiration) await this._renewCredentials()
        return this._token!
    }

    async getEntitlement(): Promise<string> {
        if(Date.now() > this._expiration) await this._renewCredentials()
        return this._entitlement!
    }

    private async _attemptReauth() {
        const response = await fetch('https://auth.riotgames.com/authorize?redirect_uri=https%3A%2F%2Fplayvalorant.com%2Fopt_in&client_id=play-valorant-web-prod&response_type=token%20id_token&nonce=1', {
            method: 'GET',
            redirect: 'manual',
            headers: {
                'User-Agent': '',
                'Cookie': createCookieString(this._cookies!)
            }
        })

        this._logger.verbose(`Response status: ${response.status}`)
        this._logger.verbose(`Response headers: ${JSON.stringify(response.headers.raw())}`)

        const location = response.headers.get('location')
        if(location === null) throw new Error('No location header in response!')
        if(!location.startsWith('https://playvalorant.com/opt_in')) {
            this._logger.error(`Reauth failed, location: ${location}`)
            throw new Error('Invalid reauth location!')
        }

        const searchParams = new URLSearchParams((new URL(response.headers.get('location')!)).hash.slice(1))

        //TODO further validation on these values
        const token = searchParams.get('access_token')
        const entitlement = searchParams.get('id_token')

        this._token = token
        this._entitlement = entitlement
        this._expiration = (Number(searchParams.get('expires_in')) * 1000) + Date.now() - expirationDiff
        this._logger.info(`Credentials refreshed, expires at ${new Date(this._expiration)}`)
        this._cookies = mergeCookies(this._cookies!, parseSetCookieString(response.headers.get('set-cookie')!))
    }

    private async _renewCredentials(): Promise<void> {
        this._logger.info('Refreshing credentials...')
        if(this._cookies === null) {
            this._cookies = parseCookieString(await fs.readFile('./cookies.txt', 'utf-8'))
            this._logger.info(`Loaded ${this._cookies.length} cookies from file`)
        }

        let delay = 0
        let attemptCount = 0
        while(attemptCount < 10) {
            attemptCount++
            try {
                await this._attemptReauth()
                break
            } catch (e) {
                this._logger.error(`Failed to reauth: ${e}`)
                this._logger.info(`Retrying in ${delay}ms...`)
                await new Promise(resolve => setTimeout(resolve, delay))
                delay = Math.min(delay + 1000, 5000)
            }
        }

        // Re-write cookies.txt
        await fs.writeFile('./cookies.txt', createCookieString(this._cookies))

        const entitlementResponse = await (await fetch('https://entitlements.auth.riotgames.com/api/token/v1', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this._token}`,
                'Content-Type': 'application/json',
                'User-Agent': ''
            }
        })).json() as {entitlements_token: string}

        this._entitlement = entitlementResponse['entitlements_token']
    }
}