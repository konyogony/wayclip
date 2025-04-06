import { Link } from 'react-router';

const Home = () => {
    return (
        <div className='flex flex-col items-center justify-center'>
            <span>To settings:</span>
            <Link to='/settings'>settings</Link>
        </div>
    );
};

export default Home;
