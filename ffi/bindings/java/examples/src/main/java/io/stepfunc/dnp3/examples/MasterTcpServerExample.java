package io.stepfunc.dnp3.examples;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import org.joou.UByte;
import org.joou.UShort;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.HashMap;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.BlockingQueue;

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
        try {
            //  We defer the handling of connections to the main loop...
            //  We can't call any of the methods on channel like addAssociation b/c this callback is from Tokio
            this.queue.put(new StartRequest(channel, destination, source));
        }
        catch (InterruptedException ignored) {
            
        }
    }
}

public class MasterTcpServerExample {
    public static void main(String[] args) throws Exception {

        Logging.configure(new LoggingConfig(), new ConsoleLogger());
        final Runtime runtime = new Runtime(new RuntimeConfig());

        final BlockingQueue<StartRequest> queue = new ArrayBlockingQueue<>(16);
        final ConnectionHandler handler = new ConnectionHandler(queue);
        final HashMap<UShort, MasterChannel> channels = new HashMap<>();

        try (MasterServer server = MasterServer.createTcpServer(runtime, "127.0.0.1:20000", new LinkIdConfig(), handler)) {
            final BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
            while(true) {
                final StartRequest request = queue.take();
                System.out.println("Accept with source == " + request.source + " destination == " + request.destination);
                final MasterChannel previous = channels.put(request.source, request.channel);
                if(previous != null) {
                    previous.shutdown();
                }
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
            }
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