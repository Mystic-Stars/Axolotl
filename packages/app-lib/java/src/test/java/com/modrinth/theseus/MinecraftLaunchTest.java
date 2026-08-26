package com.modrinth.theseus;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.junit.jupiter.api.Test;

final class MinecraftLaunchTest {
    private static final String PREFIX = "__THESEUS_UTF8__:";

    @Test
    void decodesUtf8GameArgument() {
        final String original = "E:\\Games\\Minecraft\\profiles\\Prominence\u2122 II";
        final String encoded = PREFIX + Base64.getEncoder().encodeToString(original.getBytes(StandardCharsets.UTF_8));

        assertEquals(original, MinecraftLaunch.decodeGameArgument(encoded));
    }

    @Test
    void leavesAsciiGameArgumentUnchanged() {
        assertEquals("--username", MinecraftLaunch.decodeGameArgument("--username"));
    }

    @Test
    void restoresEscapedPrefix() {
        final String original = PREFIX + "literal";
        final String encoded = PREFIX + Base64.getEncoder().encodeToString(original.getBytes(StandardCharsets.UTF_8));

        assertEquals(original, MinecraftLaunch.decodeGameArgument(encoded));
    }
}
