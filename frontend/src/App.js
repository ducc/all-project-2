import React, { Component } from 'react'
import { Chart } from 'react-google-charts'
import logo from './logo.svg'
import './App.css'

const axios = require('axios')

class ExampleChart extends Component {
  constructor(props) {
    super(props)
    this.state = {
      options: {},
      data: {},
    }
  }

  async componentDidMount() {
      let data = await axios.get('http://localhost:1337/noise_levels?from=1519975366&to=1519975379').then(res => {
        return res.data
      })

      this.setState({
        options: {
          title: 'Time vs. Noise level comparison',
          hAxis: { title: 'Time', minValue: 0 },
          vAxis: { title: 'Noise level', minValue: 0 },
          legend: 'none',
        },
        data: [['Time', 'Noise level']].concat(data),
      })

      console.log("data2: " + JSON.stringify(this.state.data));
  }

  render() {
    return (
      <Chart
        chartType='ScatterChart'
        data={this.state.data}
        options={this.state.options}
        graph_id='ScatterChart'
        width='100%'
        height='400px'
        legend_toggle
      />
    )
  }
}

class App extends Component {
  render() {
    return (
      <ExampleChart />
    )
  }
}

export default App
