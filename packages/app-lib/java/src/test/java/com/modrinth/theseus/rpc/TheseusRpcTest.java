package com.modrinth.theseus.rpc;

import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.reflect.TypeToken;
import java.io.BufferedReader;
import java.io.EOFException;
import java.io.InputStreamReader;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;

final class TheseusRpcTest {
    @Test
    void eofCompletesPendingResponsesWithoutNullDereference() throws Exception {
        try (ServerSocket server = new ServerSocket(0)) {
            final Thread peer = new Thread(() -> acceptOneRequestAndClose(server));
            peer.start();

            TheseusRpc.connectAndStart("127.0.0.1", server.getLocalPort(), new RpcHandlers());
            final ExecutionException error =
                    org.junit.jupiter.api.Assertions.assertThrows(ExecutionException.class, () -> TheseusRpc.getRpc()
                            .callMethod(TypeToken.get(String.class), "pending")
                            .get(5, TimeUnit.SECONDS));

            assertInstanceOf(EOFException.class, error.getCause());
            peer.join(TimeUnit.SECONDS.toMillis(5));
            assertTrue(!peer.isAlive());
        }
    }

    private static void acceptOneRequestAndClose(ServerSocket server) {
        try (Socket socket = server.accept();
                BufferedReader reader =
                        new BufferedReader(new InputStreamReader(socket.getInputStream(), StandardCharsets.UTF_8))) {
            reader.readLine();
        } catch (Exception error) {
            throw new RuntimeException(error);
        }
    }
}
