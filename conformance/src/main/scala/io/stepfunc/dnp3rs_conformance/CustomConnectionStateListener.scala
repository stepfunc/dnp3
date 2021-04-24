package io.stepfunc.dnp3_conformance

import io.stepfunc.dnp3.{ConnectionState, ConnectionStateListener}

class CustomConnectionStateListener extends ConnectionStateListener {
  override def onChange(connectionState: ConnectionState): Unit = {

  }
}
