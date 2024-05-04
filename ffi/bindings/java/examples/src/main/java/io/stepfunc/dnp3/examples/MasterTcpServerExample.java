package io.stepfunc.dnp3.examples;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import org.joou.UShort;

class ConnectionHandler implements io.stepfunc.dnp3.ConnectionHandler {

    @Override
    public void accept(String remoteAddr, AcceptHandler acceptor) {

    }

    @Override
    public void start(String remoteAddr, MasterChannel channel) {

    }

    @Override
    public void acceptWithLinkId(String remoteAddr, UShort source, UShort destination, IdentifiedLinkHandler acceptor) {

    }

    @Override
    public void startWithLinkId(String remoteAddr, UShort source, UShort destination, MasterChannel channel) {

    }
}

public class MasterTcpServerExample {
    public static void main(String[] args) throws Exception {

        Logging.configure(new LoggingConfig(), new ConsoleLogger());
        final Runtime runtime = new Runtime(new RuntimeConfig());

    }
}
