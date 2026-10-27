// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package io.stepfunc.dnp3.examples;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import org.joou.UByte;
import org.joou.UShort;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.util.HashMap;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.BlockingQueue;

public class MasterTcpServerExample {
    public static void main(String[] args) throws Exception {

        Logging.configure(new LoggingConfig(), new ConsoleLogger());
        final Runtime runtime = new Runtime(new RuntimeConfig());

        try {
            run(runtime);
        } finally {
            runtime.shutdown();
        }
    }

    private static void run(Runtime runtime) throws IOException, InterruptedException {
        final BlockingQueue<StartRequest> queue = new ArrayBlockingQueue<>(16);
        final ConnectionHandler handler = new ConnectionHandler(queue);

        try (MasterServer server = MasterServer.createTcpServer(runtime, "127.0.0.1:20000", new LinkIdConfig(), handler)) {
            // the channels are started on another thread, so this one can read the console
            final Thread starter = new Thread(() -> startChannels(queue));
            starter.setDaemon(true);
            starter.start();

            try {
                readConsole();
            } finally {
                // stop the starter thread before the runtime shuts down
                starter.interrupt();
                starter.join();
            }
        }
    }

    private static void readConsole() throws IOException {
        final BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        // exit on "x" or at the end of input
        while (true) {
            final String line = reader.readLine();
            if (line == null || line.equals("x")) {
                return;
            }
        }
    }

    private static void startChannels(BlockingQueue<StartRequest> queue) {
        final HashMap<UShort, MasterChannel> channels = new HashMap<>();
        try {
            while (true) {
                startChannel(queue.take(), channels);
            }
        }
        catch (InterruptedException ignored) {

        }
    }

    private static void startChannel(StartRequest request, HashMap<UShort, MasterChannel> channels) {
        System.out.println("Accept with source == " + request.source + " destination == " + request.destination);
        try {
            request.channel.addAssociation(
                    request.source,
                    new AssociationConfig(
                            EventClasses.all(),
                            EventClasses.all(),
                            Classes.all(),
                            EventClasses.none()
                    ),
                    new NullReadHandler(),
                    new NullAssocHandler(),
                    new NullAssocInfo()
            );
            request.channel.enable();
        } catch (ParamException ex) {
            // for example, the source isn't a valid outstation address
            System.out.println("channel setup failed: " + ex.getMessage());
            request.channel.shutdown();
            return;
        }
        final MasterChannel previous = channels.put(request.source, request.channel);
        if(previous != null) {
            previous.shutdown();
        }
    }
}

class StartRequest {
    final MasterChannel channel;
    final UShort destination;
    final UShort source;

    StartRequest(MasterChannel channel, UShort destination, UShort source) {
        this.channel = channel;
        this.destination = destination;
        this.source = source;
    }
}

class ConnectionHandler implements io.stepfunc.dnp3.ConnectionHandler {

    final BlockingQueue<StartRequest> queue;

    ConnectionHandler(BlockingQueue<StartRequest> queue) {
        this.queue = queue;
    }

    @Override
    public void accept(String remoteAddr, AcceptHandler acceptor) {
        acceptor.getLinkIdentity();
    }

    @Override
    public void start(String remoteAddr, MasterChannel channel) {
        // this should not be called since we always ask for the link identity
        channel.shutdown();
    }

    @Override
    public void acceptWithLinkId(String remoteAddr, UShort source, UShort destination, IdentifiedLinkHandler acceptor) {
        MasterChannelConfig config = new MasterChannelConfig(destination);
        config.decodeLevel.application = AppDecodeLevel.OBJECT_VALUES;
        acceptor.accept(LinkErrorMode.CLOSE, config);
    }

    @Override
    public void startWithLinkId(String remoteAddr, UShort source, UShort destination, MasterChannel channel) {
        //  We defer the handling of connections to the starter thread...
        //  We can't call any of the methods on channel like addAssociation b/c this callback is from Tokio
        //  Never block this callback: if the queue is full, drop the connection
        if (!this.queue.offer(new StartRequest(channel, destination, source))) {
            channel.shutdown();
        }
    }
}

class NullReadHandler implements ReadHandler {}
class NullAssocHandler implements AssociationHandler {
    @Override
    public UtcTimestamp getCurrentTime() {
        return UtcTimestamp.invalid();
    }
}
class NullAssocInfo implements AssociationInformation {

    @Override
    public void taskStart(TaskType taskType, FunctionCode functionCode, UByte seq) {

    }

    @Override
    public void taskSuccess(TaskType taskType, FunctionCode functionCode, UByte seq) {

    }

    @Override
    public void taskFail(TaskType taskType, TaskError error) {

    }

    @Override
    public void unsolicitedResponse(boolean isDuplicate, UByte seq) {

    }
}
