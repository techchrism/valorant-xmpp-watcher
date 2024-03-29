import {CredentialManager} from './CredentialManager'
import {asyncSocketRead, asyncSocketWrite, waitForConnect} from './asyncSocketUtils'
import {connect, TLSSocket} from 'node:tls'
import {Logger} from 'winston'
import {clearInterval} from 'timers'
import * as fs from 'node:fs'
import {getOrLoadRiotConfig} from './util/riotConfig'

export class XMPPManager {
    private readonly _credentialManager: CredentialManager
    private readonly _logger: Logger
    private _socket: TLSSocket | null = null
    private _requestID = 0

    constructor(credentialManager: CredentialManager, logger: Logger) {
        this._credentialManager = credentialManager
        this._logger = logger
    }

    private async _fetchPASToken() {
        const bearerToken = await this._credentialManager.getToken()
        return await (await fetch('https://riot-geo.pas.si.riotgames.com/pas/v1/service/chat', {
            headers: {
                'Authorization': `Bearer ${bearerToken}`,
                'User-Agent': ''
            }
        })).text()
    }

    private async _asyncSocketWriteLog(socket: TLSSocket, logStream: fs.WriteStream, data: string) {
        logStream.write(JSON.stringify({
            type: 'outgoing',
            time: Date.now(),
            data
        }) + '\n')
        await asyncSocketWrite(socket, data)
    }

    async connect() {
        const pasToken = await this._fetchPASToken()
        const token = await this._credentialManager.getToken()
        const entitlement = await this._credentialManager.getEntitlement()

        // Get affinity from PAS token
        const pasParts = pasToken.split('.')
        if(pasParts.length !== 3) throw new Error('Invalid PAS token')
        const pasData = JSON.parse(Buffer.from(pasParts[1], 'base64').toString('utf-8'))
        const affinity = pasData['affinity']
        if(affinity === undefined) throw new Error('Invalid PAS token, missing affinity')

        // Get affinity host and domain from riot config
        const riotConfig = await getOrLoadRiotConfig(token, entitlement)
        if(!riotConfig['chat.affinities'].hasOwnProperty(affinity)) throw new Error('PAS token affinity not found in riot config affinities')
        if(!riotConfig['chat.affinity_domains'].hasOwnProperty(affinity)) throw new Error('PAS token affinity not found in riot config affinity_domains')
        const affinityHost = riotConfig['chat.affinities'][affinity]
        const affinityDomain = riotConfig['chat.affinity_domains'][affinity]

        const xmppLogDir = './xmpp-logs'
        try {
            await fs.promises.mkdir(xmppLogDir)
        } catch(ignored) {
        }
        const xmppLogPath = `${xmppLogDir}/${Date.now()}.txt`
        const logStream = fs.createWriteStream(xmppLogPath)
        // Log header format
        logStream.write(JSON.stringify({
            type: 'valorant-xmpp-logger',
            version: '1.1.0'
        }) + '\n')

        this._logger.info('Connecting to XMPP server...')
        this._socket = connect({
            host: affinityHost,
            port: 5223
        })
        await waitForConnect(this._socket)
        this._requestID = 0

        this._socket.on('data', data => {
            logStream.write(JSON.stringify({
                type: 'incoming',
                time: Date.now(),
                data: data.toString()
            }) + '\n')
        })
        this._socket.on('error', err => {
            this._logger.warn({xmppError: err})
        })

        this._logger.info('Connected to XMPP server, authenticating...')

        await this._asyncSocketWriteLog(this._socket, logStream, `<?xml version="1.0"?><stream:stream to="${affinityDomain}.pvp.net" version="1.0" xmlns:stream="http://etherx.jabber.org/streams">`)
        let incomingData = ''
        do {
            incomingData = (await asyncSocketRead(this._socket)).toString()
        } while(!incomingData.includes('X-Riot-RSO-PAS'))

        this._logger.info('Authentication stage 2...')
        await this._asyncSocketWriteLog(this._socket, logStream, `<auth mechanism="X-Riot-RSO-PAS" xmlns="urn:ietf:params:xml:ns:xmpp-sasl"><rso_token>${token}</rso_token><pas_token>${pasToken}</pas_token></auth>`)
        await asyncSocketRead(this._socket)

        this._logger.info('Authentication stage 3...')
        await this._asyncSocketWriteLog(this._socket, logStream, `<?xml version="1.0"?><stream:stream to="${affinityDomain}.pvp.net" version="1.0" xmlns:stream="http://etherx.jabber.org/streams">`)
        do {
            incomingData = (await asyncSocketRead(this._socket)).toString()
        } while(!incomingData.includes('stream:features'))

        this._logger.info('Authentication stage 4...')
        await this._asyncSocketWriteLog(this._socket, logStream, '<iq id="_xmpp_bind1" type="set"><bind xmlns="urn:ietf:params:xml:ns:xmpp-bind"></bind></iq>')
        await asyncSocketRead(this._socket)

        this._logger.info('Authentication stage 5...')
        await this._asyncSocketWriteLog(this._socket, logStream, '<iq id="_xmpp_session1" type="set"><session xmlns="urn:ietf:params:xml:ns:xmpp-session"/></iq>')
        await asyncSocketRead(this._socket)

        this._logger.info('Authentication stage 6...')
        await this._asyncSocketWriteLog(this._socket, logStream, `<iq id="xmpp_entitlements_0" type="set"><entitlements xmlns="urn:riotgames:entitlements"><token xmlns="">${entitlement}</token></entitlements></iq>`)
        await asyncSocketRead(this._socket)

        this._logger.info('Finished authentication')

        this._logger.info('Requesting roster and chats...')
        await this._asyncSocketWriteLog(this._socket, logStream, '<iq type="get" id="1"><query xmlns="jabber:iq:riotgames:roster" last_state="true"/></iq><iq type="get" id="recent_convos_3"><query xmlns="jabber:iq:riotgames:archive:list"/></iq>')

        this._logger.info('Requesting presence...')
        await this._asyncSocketWriteLog(this._socket, logStream, '<presence/>')

        const keepAliveInterval = setInterval(async() => {
            if(this._socket !== null) {
                await this._asyncSocketWriteLog(this._socket, logStream, ' ')
            }
        }, 150_000)

        this._socket.on('close', () => {
            clearInterval(keepAliveInterval)

            // Might be a memory leak, but it probably doesn't happen often enough to matter
            this._logger.warn('XMPP connection closed, reconnecting in 5 seconds...')
            setTimeout(() => this.connect(), 5000)
        })
    }
}