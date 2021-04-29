import React from 'react';
import './App.css';

class Foo extends React.Component {
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
        var xhr = new XMLHttpRequest();
        xhr.addEventListener('load', () => {
            console.log('loaded');
            let data = JSON.parse(xhr.responseText);
            this.setState({
                data: data,
            });
        });
        xhr.open('GET', 'http://3.23.100.182:3000/state');
        xhr.send();
    }

    renderData(data) {
        let dataLen = data.length;
        let firstIdx = Math.max(dataLen - 25, 0);
        return (
            <div>
                <table className="table">
                    <tr>
                        {data.slice(firstIdx).map(e => {
                            return (
                                <td>
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
                {this.renderData(this.state.data[0])}
                {this.renderData(this.state.data[1])}
                {this.renderData(this.state.data[2])}
                {this.renderData(this.state.data[3])}
                {this.renderData(this.state.data[4])}
            </div>
        );
    }
}

class RequestButton extends React.Component {
    request() {
        var xhr = new XMLHttpRequest();
        xhr.open('POST', 'http://3.23.100.182:3000/request');
        xhr.send();
    }

    render() {
        return (
            <input type="button" value="Request" onClick={this.request} />
        );
    }
}

function App() {
  return (
      <div>
          <Foo />
          <RequestButton />
      </div>
  );
}

export default App;
