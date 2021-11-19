package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3.{ConnectionState, ConnectionStateListener}

class CustomConnectionStateListener extends ConnectionStateListener {
  override def onChange(connectionState: ConnectionState): Unit = {

  }
}
