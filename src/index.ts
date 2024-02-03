import {CredentialManager} from './CredentialManager'
import {XMPPManager} from './XMPPManager'
import * as winston from 'winston'
import 'winston-daily-rotate-file'

(async() => {
    const logger = winston.createLogger({
        level: 'silly',
        transports: [
            new winston.transports.Console({
                level: 'info',
                format: winston.format.combine(
                    winston.format.colorize(),
                    winston.format.padLevels(),
                    winston.format.simple()
                )
            }),
            new winston.transports.DailyRotateFile({
                level: 'silly',
                dirname: 'logs',
                filename: 'log-%DATE%.log',
                zippedArchive: true,
                format: winston.format.combine(
                    winston.format.timestamp(),
                    winston.format.json()
                )
            })
        ]
    })

    logger.info('Starting...')

    const credentialManager = new CredentialManager(logger)
    const xmppManager = new XMPPManager(credentialManager, logger)
    await xmppManager.connect()
})()

export {}