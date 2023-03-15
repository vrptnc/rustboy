class PwmProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.currentSample = 0
    }

    static get parameterDescriptors() {
        return [
            {
                name: "trigger",
                defaultValue: 0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "gain",
                defaultValue: 1,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "frequency",
                defaultValue: 200,
                minValue: 0,
                maxValue: 20000,
                automationRate: "k-rate",
            },
            {
                name: "dutyCycle",
                defaultValue: 0.5,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "leftChannelGain",
                defaultValue: 1.0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "rightChannelGain",
                defaultValue: 1.0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
        ];
    }

    process(inputs, outputs, parameters) {
        const enabled = parameters.trigger[0] > 0.5
        if (enabled) {
            const samplesPerWavelength = Math.floor(sampleRate / parameters.frequency[0])
            const highSamplesPerWavelength = Math.floor(samplesPerWavelength * parameters.dutyCycle[0])
            const samples = []
            for (let i = 0; i < 128; i++) {
                if (this.currentSample > samplesPerWavelength) {
                    this.currentSample = 0
                }
                const value = this.currentSample < highSamplesPerWavelength ? -1 : 1;
                samples.push(value * parameters.gain[0])
                this.currentSample++
            }
            const output = outputs[0];
            output.forEach((channel, channelIndex) => {
                const channelGain = channelIndex === 0 ? parameters.leftChannelGain[0] : parameters.rightChannelGain[0]
                for(let i = 0; i < 128; ++i) {
                    channel[i] = samples[i] * channelGain
                }
            });
        }
        return true;
    }
}

registerProcessor("pwm-processor", PwmProcessor);