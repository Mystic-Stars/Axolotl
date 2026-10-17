import { beforeAll, describe, expect, it } from 'vitest'

import { getAllCommands, initializeRegistry } from './commands/registry'
import { buildCommand, getDefaultValues, validateCommand } from './generator'

// 修复：registry 是懒加载的，测试前必须先初始化
beforeAll(() => {
    initializeRegistry()
})

describe('buildCommand', () => {
    it('生成基础 give 命令', () => {
        expect(
            buildCommand('give', {
                target: '@s',
                item: 'minecraft:diamond',
                count: 64,
            }),
        ).toBe('/give @s minecraft:diamond 64')
    })

    it('生成 summon 命令带 NBT', () => {
        expect(
            buildCommand('summon', {
                entity: 'minecraft:axolotl',
                pos: '~ ~ ~',
                nbt: '{Variant:0}',
            }),
        ).toBe('/summon minecraft:axolotl ~ ~ ~ {Variant:0}')
    })

    it('生成无参数命令', () => {
        expect(buildCommand('seed', {})).toBe('/seed')
        expect(buildCommand('reload', {})).toBe('/reload')
    })

    it('布尔参数仅在 true 时附加', () => {
        expect(buildCommand('list', { uuids: true })).toBe('/list uuids')
        expect(buildCommand('list', { uuids: false })).toBe('/list')
    })

    it('消息参数自动加引号', () => {
        expect(buildCommand('say', { message: 'Hello World' })).toBe('/say "Hello World"')
    })

    it('跳过空的可选参数', () => {
        expect(buildCommand('gamemode', { mode: 'creative', target: '' })).toBe(
            '/gamemode creative',
        )
    })

    it('必填参数缺失时抛出错误', () => {
        expect(() => buildCommand('gamemode', {})).toThrow()
    })

    it('未知指令抛出错误', () => {
        expect(() => buildCommand('nonexistent', {})).toThrow('Unknown command')
    })
})

describe('validateCommand', () => {
    it('有效参数返回空错误数组', () => {
        expect(validateCommand('gamemode', { mode: 'creative' })).toHaveLength(0)
    })

    it('必填项缺失返回错误', () => {
        expect(validateCommand('give', {}).length).toBeGreaterThan(0)
    })

    it('枚举值非法返回错误', () => {
        expect(validateCommand('gamemode', { mode: 'invalid_mode' }).length).toBeGreaterThan(0)
    })

    it('整数参数传入非整数返回错误', () => {
        expect(
            validateCommand('give', {
                target: '@s',
                item: 'minecraft:stone',
                count: 1.5,
            }).length,
        ).toBeGreaterThan(0)
    })
})

describe('getDefaultValues', () => {
    it('返回预设默认值', () => {
        expect(getDefaultValues('give').count).toBe(1)
    })

    it('未知指令返回空对象', () => {
        expect(getDefaultValues('nonexistent')).toEqual({})
    })
})

describe('registry completeness', () => {
    it('所有指令都有唯一的 id', () => {
        const commands = getAllCommands()
        const ids = commands.map((c) => c.id)
        expect(new Set(ids).size).toBe(ids.length)
    })

    it('所有指令都有描述和分类', () => {
        // 修复：字段名是 descriptionKey 而不是 description
        for (const cmd of getAllCommands()) {
            expect(cmd.descriptionKey).toBeTruthy()
            expect(cmd.category).toBeTruthy()
        }
    })
})
