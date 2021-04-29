import React from 'react';
import Button from 'react-bootstrap/Button';
import './App.css';

class LogView extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            data: [[],[],[],[],[]],
        };
    }

    componentDidMount() {
        this.timer = setInterval(
            () => this.tick(),
            250
        );
    }

    componentWillUnmount() {
        clearInterval(this.timer);
    }

    tick() {
        fetch('https://raftviz.herokuapp.com/state')
            .then(res => res.json())
            .then(data => this.setState({data: data}));
    }

    renderData(data, idx) {
        let dataLen = data[idx].length;
        let firstIdx = Math.max(dataLen - 25, 0);
        return (
            <div>
                <table className="table table-bordered">
                    <tr>
                        <th>Server {idx}</th>
                        <th>
                            Term
                            <br />
                            Opid
                        </th>
                        {data[idx].slice(firstIdx).map(e => {
                            return (
                                <td>
                                    {e['term']}
                                    <br />
                                    {e['opid']}
                                </td>
                            );
                        })}
                    </tr>
                </table>
            </div>
        );
    }

    render() {
        return (
            <div>
                {this.renderData(this.state.data, 0)}
                {this.renderData(this.state.data, 1)}
                {this.renderData(this.state.data, 2)}
                {this.renderData(this.state.data, 3)}
                {this.renderData(this.state.data, 4)}
            </div>
        );
    }
}

class RequestButton extends React.Component {
    request() {
        fetch('https://raftviz.herokuapp.com/request', {method:'POST'});
    }

    render() {
        return (
            <Button size="lg" onClick={this.request}>Make a Request</Button>
        );
    }
}

function App() {
  return (
      <div style={{margin: '25px'}}>
          <center>
              <LogView />
              <RequestButton />
          </center>
      </div>
  );
}

export default App;
