class GBAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.queue = [];
    this.head = 0;
    this.samplesSinceLastReport = 0;
    this.underrunsSinceLastReport = 0;

    this.port.onmessage = (event) => {
      const data = event.data;
      if (!data) {
        return;
      }

      if (data.type === "samples") {
        this.pushSamples(data.samples);
      } else if (data.type === "reset") {
        this.queue.length = 0;
        this.head = 0;
        this.samplesSinceLastReport = 0;
        this.underrunsSinceLastReport = 0;
      }
    };
  }

  pushSamples(samples) {
    if (!samples || samples.length === 0) {
      return;
    }

    if (samples instanceof Float32Array) {
      this.queue.push(samples);
    } else {
      this.queue.push(new Float32Array(samples));
    }
  }

  popSample() {
    while (this.queue.length > 0) {
      const front = this.queue[0];
      if (this.head < front.length) {
        const value = front[this.head];
        this.head += 1;
        return value;
      }
      this.queue.shift();
      this.head = 0;
    }

    return null;
  }

  process(_inputs, outputs) {
    const output = outputs[0];
    if (!output || output.length === 0) {
      return true;
    }

    const channel0 = output[0];
    for (let i = 0; i < channel0.length; i += 1) {
      const sample = this.popSample();
      if (sample === null) {
        channel0[i] = 0.0;
        this.underrunsSinceLastReport += 1;
      } else {
        channel0[i] = sample;
      }
    }

    for (let channel = 1; channel < output.length; channel += 1) {
      output[channel].set(channel0);
    }

    this.samplesSinceLastReport += channel0.length;
    if (this.samplesSinceLastReport >= 128) {
      this.port.postMessage({
        type: "consumed",
        samples: this.samplesSinceLastReport,
        underruns: this.underrunsSinceLastReport,
      });
      this.samplesSinceLastReport = 0;
      this.underrunsSinceLastReport = 0;
    }

    return true;
  }
}

registerProcessor("gb-audio-processor", GBAudioProcessor);
