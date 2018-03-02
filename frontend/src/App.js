import React, { Component } from 'react'
import { Chart } from 'react-google-charts'
import logo from './logo.svg'
import './App.css'

class ExampleChart extends Component {
  constructor(props) {
    super(props)
    this.state = {
      options: {
        title: 'Age vs. Weight comparison',
        hAxis: { title: 'Age', minValue: 0, maxValue: 15 },
        vAxis: { title: 'Weight', minValue: 0, maxValue: 15 },
        legend: 'none',
      },
      data: [
        ['Age', 'Weight'],
        [8, 12],
        [4, 5.5],
        [11, 14],
        [4, 5],
        [3, 3.5],
        [6.5, 7],
      ]
    }
  }

  render() {
    return (
      <Chart
        chartType="ScatterChart"
        data={this.state.data}
        options={this.state.options}
        graph_id="ScatterChart"
        width="100%"
        height="400px"
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
